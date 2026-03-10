import DOMPurify from "dompurify";
import { marked } from "marked";
import { Message, ToolRow } from "../../../../codex-gui/models";

marked.setOptions({
    breaks: true,
    gfm: true,
});

export type CodexGuiRenderedMessage = {
    renderKind: "user" | "tool" | "standard" | "reasoning";
    html: string;
    plainContent: string;
    streamingPlain: boolean;
    toolRows: ToolRow[];
    isToolRunning: boolean;
    toolStatusLabel: string;
    showStreamingIndicator: boolean;
};

export function renderCodexGuiMessage(
    message: Message,
    stripPathPrefix: string,
): CodexGuiRenderedMessage {
    const presentation = message.presentation;
    const text = presentation.text ?? message.content;
    const renderKind = presentation.kind;

    return {
        renderKind,
        html:
            renderKind === "tool"
                ? ""
                : renderKind === "reasoning"
                  ? renderReasoningContent(text)
                  : renderTextContent(text, stripPathPrefix),
        plainContent: text,
        streamingPlain: message.status === "streaming" && renderKind !== "user",
        toolRows: presentation.toolRows ?? [],
        isToolRunning: presentation.isToolRunning ?? false,
        toolStatusLabel: presentation.toolStatusLabel ?? "",
        showStreamingIndicator: message.status === "streaming" && renderKind !== "tool",
    };
}

export function shouldShowGlobalTypingIndicator(
    messages: readonly Message[],
    isWorking: boolean,
): boolean {
    if (!isWorking) {
        return false;
    }
    const latest = messages[messages.length - 1];
    if (!latest) {
        return true;
    }
    if (latest.status === "streaming") {
        return false;
    }
    return !(latest.presentation.kind === "tool" && latest.presentation.isToolRunning);
}

export function globalTypingLabel(messages: readonly Message[]): string {
    const latest = messages[messages.length - 1];
    if (!latest) {
        return "Working";
    }

    if (latest.presentation.kind === "tool") {
        const firstRow = latest.presentation.toolRows[0];
        if (firstRow?.kind === "command" && firstRow.value) {
            return firstRow.value.trim() || "Running command";
        }
        if (firstRow?.kind === "search" && firstRow.value) {
            return `Searching ${firstRow.value}`;
        }
        if (firstRow?.kind === "read" && firstRow.path) {
            return "Reading file";
        }
        if (firstRow?.label) {
            return firstRow.label;
        }
    }

    if (latest.presentation.kind === "reasoning" && latest.status === "streaming") {
        return "Thinking";
    }
    if (latest.role === "assistant" && latest.status === "streaming") {
        return "Thinking";
    }
    return "Working";
}

function renderTextContent(content: string, stripPathPrefix: string): string {
    const rawHtml = marked.parse(content) as string;
    const sanitized = DOMPurify.sanitize(rawHtml, {
        ALLOWED_TAGS: [
            "p",
            "br",
            "strong",
            "em",
            "code",
            "pre",
            "blockquote",
            "ul",
            "ol",
            "li",
            "a",
            "h1",
            "h2",
            "h3",
            "h4",
            "h5",
            "h6",
            "table",
            "thead",
            "tbody",
            "tr",
            "th",
            "td",
            "hr",
        ],
        ALLOWED_ATTR: ["href", "target", "rel"],
    });
    return applyDisplayPaths(sanitized, stripPathPrefix);
}

function renderReasoningContent(content: string): string {
    const normalized = content
        .split("\n")
        .map((line) => line.trim())
        .filter((line) => line.length > 0)
        .join("<br>");
    return DOMPurify.sanitize(normalized, {
        ALLOWED_TAGS: ["br"],
        ALLOWED_ATTR: [],
    });
}

function applyDisplayPaths(html: string, stripPathPrefix: string): string {
    const prefix = stripPathPrefix.trim();
    const container = document.createElement("div");
    container.innerHTML = html;

    for (const anchor of Array.from(container.querySelectorAll("a"))) {
        const href = anchor.getAttribute("href") ?? "";
        const path = pathFromAnchorHref(href);
        if (!path) {
            continue;
        }
        const displayPath = toDisplayPath(path, prefix);
        const currentText = (anchor.textContent ?? "").trim();
        if (!currentText || currentText === path || currentText === href) {
            anchor.textContent = displayPath;
        }
    }
    return container.innerHTML;
}

function pathFromAnchorHref(href: string): string | null {
    if (href.startsWith("#diff:")) {
        const encodedPath = href.slice("#diff:".length);
        if (!encodedPath) {
            return null;
        }
        try {
            return decodeURIComponent(encodedPath);
        } catch {
            return encodedPath;
        }
    }
    if (href.startsWith("/")) {
        return href;
    }
    if (href.startsWith("file://")) {
        try {
            return decodeURIComponent(href.slice("file://".length));
        } catch {
            return href.slice("file://".length);
        }
    }
    return null;
}

export function toDisplayPath(path: string, prefix: string): string {
    const normalizedPath = path.replaceAll("\\", "/");
    if (!prefix) {
        return normalizedPath;
    }
    for (const candidate of matchablePathPrefixes(prefix)) {
        const normalizedPrefix = candidate.endsWith("/")
            ? candidate
            : `${candidate}/`;
        if (normalizedPath.startsWith(normalizedPrefix)) {
            return normalizedPath.slice(normalizedPrefix.length);
        }
    }
    return normalizedPath;
}

function matchablePathPrefixes(prefix: string): string[] {
    const normalized = prefix.trim().replaceAll("\\", "/").replace(/\/+$/, "");
    if (!normalized) {
        return [];
    }

    const prefixes = new Set<string>([normalized]);

    const windowsMatch = normalized.match(/^([a-zA-Z]):\/(.*)$/);
    if (windowsMatch) {
        const drive = windowsMatch[1].toLowerCase();
        const rest = windowsMatch[2].replace(/^\/+/, "");
        prefixes.add(`/mnt/${drive}${rest ? `/${rest}` : ""}`);
    }

    const wslMatch = normalized.match(/^\/mnt\/([a-zA-Z])(?:\/(.*))?$/);
    if (wslMatch) {
        const drive = wslMatch[1].toUpperCase();
        const rest = (wslMatch[2] ?? "").replace(/^\/+/, "");
        prefixes.add(`${drive}:/${rest}`.replace(/\/+$/, ""));
    }

    return Array.from(prefixes);
}
