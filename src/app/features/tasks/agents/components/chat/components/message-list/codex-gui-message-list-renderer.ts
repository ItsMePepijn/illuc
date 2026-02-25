import DOMPurify from "dompurify";
import { marked } from "marked";
import { Message } from "../../../../models";

marked.setOptions({
    breaks: true,
    gfm: true,
});

export type CodexGuiRenderedMessage = {
    renderKind: "user" | "tool" | "standard" | "reasoning";
    html: string;
    plainContent: string;
    streamingPlain: boolean;
    toolRowsHtml: string[];
    isToolRunning: boolean;
    toolStatusLabel: string;
    showStreamingIndicator: boolean;
};

export function renderCodexGuiMessage(
    message: Message,
    stripPathPrefix: string,
): CodexGuiRenderedMessage {
    const toolMessage = isCodexGuiToolMessage(message);
    const reasoningMessage = isCodexGuiReasoningMessage(message);
    const statusLabel = toolMessage ? codexGuiToolStatusLabel(message) : "";
    const reasoningContent = reasoningMessage
        ? codexGuiReasoningText(message.content)
        : message.content;

    return {
        renderKind:
            message.role === "user"
                ? "user"
                : toolMessage
                  ? "tool"
                  : reasoningMessage
                    ? "reasoning"
                  : "standard",
        html:
            message.role === "user" || !toolMessage
                ? reasoningMessage
                  ? renderReasoningContent(reasoningContent)
                  : renderContent(message.content, stripPathPrefix)
                : "",
        plainContent: reasoningContent,
        streamingPlain: message.status === "streaming" && message.role !== "user",
        toolRowsHtml: toolMessage
            ? codexGuiToolRows(message).map((row) =>
                  renderToolRow(row, stripPathPrefix),
              )
            : [],
        isToolRunning: statusLabel === "RUNNING",
        toolStatusLabel: statusLabel,
        showStreamingIndicator: message.status === "streaming" && !toolMessage,
    };
}

export function isCodexGuiReasoningMessage(message: Message): boolean {
    if (message.role !== "system") {
        return false;
    }
    const text = message.content.trim();
    return text === "Reasoning" || text.startsWith("Reasoning\n");
}

export function isCodexGuiToolMessage(message: Message): boolean {
    if (message.role !== "system") {
        return false;
    }
    const text = message.content.trim();
    return (
        message.status === "streaming" ||
        text.startsWith("$ ") ||
        text.startsWith("File changes [") ||
        text.startsWith("- Read ") ||
        text.startsWith("- Edited ") ||
        text.startsWith("- Created ") ||
        text.startsWith("- Deleted ") ||
        text.startsWith("- Renamed ") ||
        text.startsWith("- ") ||
        /\[(completed|failed|declined|inProgress)(?:\s+exit\s+[-\d?]+)?\]/i.test(text)
    );
}

export function codexGuiToolStatusLabel(message: Message): string {
    const match = message.content.match(
        /\[(completed|failed|declined|inProgress)(?:\s+exit\s+([-\d?]+))?\]/i,
    );
    if (!match) {
        return message.status === "streaming" ? "RUNNING" : "";
    }
    const value = match[1].toLowerCase();
    if (value === "inprogress") {
        return "RUNNING";
    }
    if (value === "completed") {
        const exitCode = match[2] ?? "0";
        if (exitCode === "0") {
            return "";
        }
        return "FAILED";
    }
    if (value === "failed") {
        return "FAILED";
    }
    return "DECLINED";
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
    return !(
        isCodexGuiToolMessage(latest) &&
        codexGuiToolStatusLabel(latest) === "RUNNING"
    );
}

export function globalTypingLabel(messages: readonly Message[]): string {
    const latest = messages[messages.length - 1];
    if (!latest) {
        return "Working";
    }
    if (isCodexGuiToolMessage(latest)) {
        const firstRow = codexGuiToolRows(latest)[0] ?? "";
        const compact = firstRow
            .replace(/^-+\s+/, "")
            .replace(/\s+/g, " ")
            .trim();
        if (compact.startsWith("$ ")) {
            return compact.slice(2).trim() || "Running command";
        }
        if (compact) {
            return compact;
        }
    }
    if (latest.role === "assistant" && latest.status === "streaming") {
        return "Thinking";
    }
    return "Working";
}

function renderContent(content: string, stripPathPrefix: string): string {
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
        .map((line) =>
            stripReasoningBoldWrapper(line.trim().replace(/^-+\s*/, "")),
        )
        .filter((line) => line.length > 0)
        .join("<br>");
    return DOMPurify.sanitize(normalized, {
        ALLOWED_TAGS: ["br"],
        ALLOWED_ATTR: [],
    });
}

function codexGuiReasoningText(content: string): string {
    const trimmed = content.trim();
    if (trimmed === "Reasoning") {
        return "";
    }
    const withoutHeader = trimmed.startsWith("Reasoning\n")
        ? trimmed.slice("Reasoning\n".length)
        : content;
    const normalized = withoutHeader
        .split("\n")
        .map((line) => stripReasoningBoldWrapper(line))
        .join("\n");
    return normalized;
}

function stripReasoningBoldWrapper(value: string): string {
    const trimmed = value.trim();
    if (trimmed.startsWith("**") && trimmed.endsWith("**") && trimmed.length > 4) {
        return trimmed.slice(2, -2).trim();
    }
    return trimmed;
}

function renderInline(content: string): string {
    const rawHtml = marked.parseInline(content) as string;
    return DOMPurify.sanitize(rawHtml, {
        ALLOWED_TAGS: ["code", "a", "strong", "em", "span"],
        ALLOWED_ATTR: ["href", "target", "rel", "class"],
    });
}

function codexGuiToolRows(message: Message): string[] {
    const text = message.content.trim();
    const patchRows = patchToolRows(text);
    if (patchRows.length > 0) {
        return patchRows;
    }
    if (text.startsWith("File changes [")) {
        return text
            .split("\n")
            .slice(1)
            .map((line) => line.trim())
            .filter((line) => line.startsWith("- "));
    }
    return text
        .split("\n")
        .map((line) => normalizeReadCommandRow(line.trim()))
        .filter(
            (line) =>
                line.length > 0 &&
                !/^\[(completed|failed|declined|inProgress)(?:\s+exit\s+[-\d?]+)?\]$/i.test(
                    line,
                ),
        );
}

function normalizeReadCommandRow(row: string): string {
    if (!row.startsWith("$ ")) {
        return row;
    }

    const command = row.slice(2).trim();
    const catMatch = command.match(/^cat\s+(.+)$/);
    if (catMatch) {
        return `- Read ${catMatch[1].trim()}`;
    }

    const sedMatch = command.match(/^sed\s+-n\s+(['"])\d+,\d+p\1\s+(.+)$/);
    if (sedMatch) {
        return `- Read ${sedMatch[2].trim()}`;
    }

    return row;
}

function patchToolRows(text: string): string[] {
    if (!text.includes("apply_patch <<")) {
        return [];
    }

    const rows: string[] = [];
    const seen = new Set<string>();
    const pushRow = (label: string, path: string): void => {
        const normalizedPath = path.trim();
        if (!normalizedPath) {
            return;
        }
        const key = `${label}:${normalizedPath}`;
        if (seen.has(key)) {
            return;
        }
        seen.add(key);
        rows.push(`- ${label} ${normalizedPath}`);
    };

    for (const match of text.matchAll(/^\*\*\* Update File: (.+)$/gm)) {
        pushRow("Edited", match[1]);
    }
    for (const match of text.matchAll(/^\*\*\* Add File: (.+)$/gm)) {
        pushRow("Created", match[1]);
    }
    for (const match of text.matchAll(/^\*\*\* Delete File: (.+)$/gm)) {
        pushRow("Deleted", match[1]);
    }
    for (const match of text.matchAll(/^\*\*\* Move to: (.+)$/gm)) {
        pushRow("Renamed", match[1]);
    }

    return rows;
}

function renderToolRow(row: string, stripPathPrefix: string): string {
    const trimmed = row.trim();
    if (!trimmed.startsWith("$ ")) {
        const withoutBullet = trimmed.replace(/^-+\s+/, "");
        const inline = applyDisplayPaths(
            renderInline(withoutBullet),
            stripPathPrefix,
        );
        const highlighted = inline.replace(
            /\(\+(\d+)\s+-(\d+)\)/g,
            "(<span class=\"t-added\">+$1</span> <span class=\"t-removed\">-$2</span>)",
        );
        return DOMPurify.sanitize(highlighted, {
            ALLOWED_TAGS: ["code", "a", "strong", "em", "span"],
            ALLOWED_ATTR: ["href", "target", "rel", "class"],
        });
    }
    const command = trimmed.slice(2).trim();
    const tokens = command.match(/"[^"]*"|'[^']*'|\S+/g) ?? [];
    if (tokens.length === 0) {
        return "";
    }
    const html = tokens
        .map((token, index) => {
            const escaped = escapeHtml(token);
            if (index === 0) {
                return `<span class="t-bin">${escaped}</span>`;
            }
            if (token.startsWith("-")) {
                return `<span class="t-flag">${escaped}</span>`;
            }
            if (
                (token.startsWith("\"") && token.endsWith("\"")) ||
                (token.startsWith("'") && token.endsWith("'"))
            ) {
                return `<span class="t-str">${escaped}</span>`;
            }
            return `<span class="t-arg">${escaped}</span>`;
        })
        .join(" ");
    return DOMPurify.sanitize(html, {
        ALLOWED_TAGS: ["span"],
        ALLOWED_ATTR: ["class"],
    });
}

function escapeHtml(value: string): string {
    return value
        .replaceAll("&", "&amp;")
        .replaceAll("<", "&lt;")
        .replaceAll(">", "&gt;");
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

function toDisplayPath(path: string, prefix: string): string {
    const normalizedPath = path.replaceAll("\\", "/");
    if (!prefix) {
        return normalizedPath;
    }
    const normalizedPrefix = prefix.replaceAll("\\", "/").endsWith("/")
        ? prefix.replaceAll("\\", "/")
        : `${prefix.replaceAll("\\", "/")}/`;
    return normalizedPath.startsWith(normalizedPrefix)
        ? normalizedPath.slice(normalizedPrefix.length)
        : normalizedPath;
}
