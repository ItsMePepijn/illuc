import { CommonModule } from "@angular/common";
import {
    ChangeDetectorRef,
    ChangeDetectionStrategy,
    Component,
    ElementRef,
    EventEmitter,
    Input,
    NgZone,
    OnChanges,
    OnDestroy,
    Output,
    SimpleChanges,
    ViewChild,
} from "@angular/core";
import {
    DynamicSizeVirtualScrollStrategy,
    RxVirtualFor,
    RxVirtualScrollElementDirective,
    RxVirtualScrollViewportComponent,
} from "@rx-angular/template/virtual-scrolling";
import { Message } from "../../../../models";
import { LoadingSpinnerComponent } from "../../../../../../../shared/components/loading-spinner/loading-spinner.component";
import {
    codexGuiToolStatusLabel,
    globalTypingLabel,
    isCodexGuiToolMessage,
    renderCodexGuiMessage,
    shouldShowGlobalTypingIndicator,
} from "./codex-gui-message-list-renderer";
import {
    CodexGuiBottomFollowController,
    CodexGuiBottomPinController,
    CodexGuiBottomSyncController,
} from "./codex-gui-message-list-scroll-controllers";
import { TypingIndicatorComponent } from "../typing-indicator/typing-indicator.component";
import { ThrobberComponent } from "../throbber/throbber.component";
import { ReasoningMessageComponent } from "./components/reasoning-message/reasoning-message.component";
import { StandardMessageComponent } from "./components/standard-message/standard-message.component";
import { ToolMessageComponent } from "./components/tool-message/tool-message.component";
import { UserMessageComponent } from "./components/user-message/user-message.component";
import { Subject, Subscription } from "rxjs";

type CodexGuiListItem = {
    key: string;
    trackKey: string;
    rowRole: "user" | "assistant";
    dataRole: "user" | "assistant" | "system" | "reasoning";
    dataStatus: "streaming" | "complete" | "error";
    renderKind: "user" | "tool" | "standard" | "reasoning" | "typing";
    html: string;
    plainContent: string;
    streamingPlain: boolean;
    toolRowsHtml: string[];
    isToolRunning: boolean;
    toolStatusLabel: string;
    showStreamingIndicator: boolean;
    typingLabel: string;
    typingStartedAt: string | null;
    showTypingLabel: boolean;
    compactWithNext: boolean;
};

type CachedMessageItem = {
    sourceKey: string;
    revision: number;
    item: CodexGuiListItem;
};

const STICK_THRESHOLD_PX = 64;
const CHAT_RUNWAY_ITEMS = 10;
const CHAT_RUNWAY_ITEMS_OPPOSITE = 2;
const CHAT_TEMPLATE_CACHE_SIZE = 0;
const CHAT_BOTTOM_SNAP_ATTEMPTS = 24;
const CHAT_ACTIVATION_SYNC_DELAYS_MS = [0, 40, 120, 280];
const CHAT_ROW_PADDING_PX = 20;
const CHAT_MIN_USER_ROW_HEIGHT_PX = 60;
const CHAT_MIN_STANDARD_ROW_HEIGHT_PX = 68;
const CHAT_MIN_REASONING_ROW_HEIGHT_PX = 44;
const CHAT_MIN_TOOL_ROW_HEIGHT_PX = 36;

function clampHeight(value: number, min: number, max: number): number {
    return Math.max(min, Math.min(max, Math.ceil(value)));
}

function countMatches(source: string, pattern: RegExp): number {
    return source.match(pattern)?.length ?? 0;
}

function stripHtmlTags(html: string): string {
    return html
        .replace(/<br\s*\/?>/gi, "\n")
        .replace(/<\/p>/gi, "\n")
        .replace(/<[^>]+>/g, " ")
        .replace(/&nbsp;/g, " ")
        .replace(/&amp;/g, "&")
        .replace(/&lt;/g, "<")
        .replace(/&gt;/g, ">")
        .replace(/\s+\n/g, "\n")
        .replace(/\n\s+/g, "\n")
        .replace(/[ \t]{2,}/g, " ")
        .trim();
}

function estimateWrappedLineCount(text: string, charsPerLine: number): number {
    const normalized = text.replace(/\r/g, "").trim();
    if (!normalized) {
        return 1;
    }
    return normalized
        .split("\n")
        .reduce(
            (lineCount, line) =>
                lineCount + Math.max(1, Math.ceil(line.length / charsPerLine)),
            0,
        );
}

@Component({
    selector: "app-codex-gui-message-list",
    standalone: true,
    imports: [
        CommonModule,
        RxVirtualFor,
        RxVirtualScrollViewportComponent,
        RxVirtualScrollElementDirective,
        DynamicSizeVirtualScrollStrategy,
        LoadingSpinnerComponent,
        TypingIndicatorComponent,
        ThrobberComponent,
        UserMessageComponent,
        ToolMessageComponent,
        ReasoningMessageComponent,
        StandardMessageComponent,
    ],
    templateUrl: "./message-list.component.html",
    styleUrl: "./message-list.component.css",
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class MessageListComponent implements OnChanges, OnDestroy {
    @Input() messages: Message[] = [];
    @Input() showInitialLoading = false;
    @Input() isWorking = false;
    @Input() activityLabel = "";
    @Input() activityStartedAt: string | null = null;
    @Input() stripPathPrefix = "";
    @Input() isActive = true;
    @Output() diffFileRequested = new EventEmitter<string>();

    @ViewChild(RxVirtualScrollViewportComponent)
    set viewportRef(
        viewport: RxVirtualScrollViewportComponent | undefined,
    ) {
        this.viewport = viewport;
        this.flushPendingScrollToBottom();
    }

    @ViewChild("scrollHost")
    set scrollHostRef(host: ElementRef<HTMLElement> | undefined) {
        this.scrollHost = host?.nativeElement;
        this.bindScrollHost();
        this.flushPendingScrollToBottom();
    }

    readonly runwayItems = CHAT_RUNWAY_ITEMS;
    readonly runwayItemsOpposite = CHAT_RUNWAY_ITEMS_OPPOSITE;
    readonly templateCacheSize = CHAT_TEMPLATE_CACHE_SIZE;
    readonly estimateHistoryItemSize = (item: CodexGuiListItem): number =>
        this.estimateItemSize(item);
    readonly historyRenderCallback = new Subject<CodexGuiListItem[]>();

    historyListItems: CodexGuiListItem[] = [];
    tailItems: CodexGuiListItem[] = [];
    initialScrollIndex = 0;

    private readonly messageItemCache = new Map<string, CachedMessageItem>();
    private readonly measuredHistoryItemHeights = new Map<string, number>();
    private readonly bottomPinController: CodexGuiBottomPinController;
    private readonly bottomFollowController: CodexGuiBottomFollowController;
    private readonly bottomSyncController: CodexGuiBottomSyncController;
    private viewport?: RxVirtualScrollViewportComponent;
    private scrollHost?: HTMLElement;
    private removeScrollListener?: () => void;
    private resizeObserver?: ResizeObserver;
    private resizeSyncFrameId?: number;
    private historyRenderSubscription?: Subscription;
    private historyMeasurementFrameId?: number;

    constructor(
        private readonly zone: NgZone,
        private readonly cdr: ChangeDetectorRef,
    ) {
        this.bottomPinController = new CodexGuiBottomPinController(
            STICK_THRESHOLD_PX,
        );
        this.bottomFollowController = new CodexGuiBottomFollowController(
            zone,
            () => this.bottomPinController.pin(),
        );
        this.bottomSyncController = new CodexGuiBottomSyncController(
            zone,
            CHAT_BOTTOM_SNAP_ATTEMPTS,
            CHAT_ACTIVATION_SYNC_DELAYS_MS,
            () => this.bottomPinController.pin(),
        );
        this.historyRenderSubscription = this.historyRenderCallback.subscribe(() => {
            this.scheduleHistoryMeasurement();
        });
    }

    ngOnChanges(changes: SimpleChanges): void {
        if (changes["stripPathPrefix"]) {
            this.messageItemCache.clear();
        }

        if (
            changes["messages"] ||
            changes["isWorking"] ||
            changes["activityLabel"] ||
            changes["activityStartedAt"] ||
            changes["stripPathPrefix"]
        ) {
            const previousHistoryLength = this.historyListItems.length;
            const previousTailAnchorKey = this.tailItems[0]?.key;

            this.rebuildItems();

            if (this.bottomPinController.isPinned) {
                if (
                    this.shouldUseBottomFollow(
                        previousTailAnchorKey,
                        previousHistoryLength,
                    )
                ) {
                    this.requestBottomFollow();
                } else {
                    this.requestBottomSync();
                }
            }
        }

        if (changes["isActive"]?.currentValue) {
            this.bottomPinController.pin();
            this.requestBottomSync();
            this.scheduleActivationBottomSync();
        } else if (changes["isActive"]) {
            this.clearActivationBottomSync();
            this.clearPendingBottomSnap();
        }
    }

    ngOnDestroy(): void {
        this.removeScrollListener?.();
        this.disconnectResizeObserver();
        this.clearActivationBottomSync();
        this.clearPendingBottomSnap();
        this.historyRenderSubscription?.unsubscribe();
        this.historyRenderCallback.complete();
        this.cancelHistoryMeasurement();
    }

    trackByItem(_index: number, item: CodexGuiListItem): string {
        return item.trackKey;
    }

    onContentClick(event: MouseEvent): void {
        const target = event.target as HTMLElement | null;
        const anchor = target?.closest("a");
        if (!anchor) {
            return;
        }
        const href = anchor.getAttribute("href") ?? "";
        const path = this.pathFromAnchorHref(href);
        if (!path) {
            return;
        }
        event.preventDefault();
        event.stopPropagation();
        this.diffFileRequested.emit(this.normalizeDiffPath(path));
    }

    private rebuildItems(): void {
        let liveTailIndex = -1;
        for (let index = 0; index < this.messages.length; index += 1) {
            if (this.messages[index]?.status === "streaming") {
                liveTailIndex = index;
                break;
            }
        }
        const historyMessages =
            liveTailIndex >= 0
                ? this.messages.slice(0, liveTailIndex)
                : this.messages;
        const liveTailMessages =
            liveTailIndex >= 0 ? this.messages.slice(liveTailIndex) : [];

        this.historyListItems = historyMessages.map((message) =>
            this.buildMessageItem(message),
        );
        this.tailItems =
            liveTailMessages.length > 0
                ? liveTailMessages.map((message) => this.buildMessageItem(message))
                : this.buildTypingIndicatorItems();
        this.applyToolSpacingCompaction();
        this.initialScrollIndex = Math.max(this.historyListItems.length - 1, 0);
        this.pruneMeasuredHistoryItemHeights();
    }

    private applyToolSpacingCompaction(): void {
        for (let index = 0; index < this.historyListItems.length; index += 1) {
            const current = this.historyListItems[index];
            const next =
                this.historyListItems[index + 1] ??
                (index === this.historyListItems.length - 1
                    ? this.tailItems[0] ?? null
                    : null);
            current.compactWithNext =
                current.renderKind === "tool" && next?.renderKind === "tool";
        }
        for (let index = 0; index < this.tailItems.length; index += 1) {
            const current = this.tailItems[index];
            const next = this.tailItems[index + 1] ?? null;
            current.compactWithNext =
                current.renderKind === "tool" && next?.renderKind === "tool";
        }
    }

    private estimateItemSize(item: CodexGuiListItem): number {
        const measuredHeight = this.measuredHistoryItemHeights.get(item.trackKey);
        if (measuredHeight !== undefined) {
            return measuredHeight;
        }
        switch (item.renderKind) {
            case "user":
                return this.estimateUserMessageHeight(item);
            case "tool":
                return this.estimateToolMessageHeight(item);
            case "reasoning":
                return this.estimateReasoningMessageHeight(item);
            case "typing":
                return 56;
            case "standard":
            default:
                return this.estimateStandardMessageHeight(item);
        }
    }

    private estimateUserMessageHeight(item: CodexGuiListItem): number {
        const text = stripHtmlTags(item.html);
        const wrappedLines = estimateWrappedLineCount(text, 42);
        const codeBlockCount = countMatches(item.html, /<pre\b/gi);
        const listCount = countMatches(item.html, /<(ul|ol)\b/gi);
        const height =
            CHAT_ROW_PADDING_PX +
            16 +
            wrappedLines * 26 +
            codeBlockCount * 56 +
            listCount * 20;
        return clampHeight(height, CHAT_MIN_USER_ROW_HEIGHT_PX, 480);
    }

    private estimateToolMessageHeight(item: CodexGuiListItem): number {
        const rowHeights = item.toolRowsHtml.reduce((height, rowHtml) => {
            const wrappedLines = estimateWrappedLineCount(
                stripHtmlTags(rowHtml),
                72,
            );
            return height + 18 + Math.max(0, wrappedLines - 1) * 18;
        }, 0);
        const height =
            CHAT_ROW_PADDING_PX +
            rowHeights +
            Math.max(0, item.toolRowsHtml.length - 1) * 2 +
            (item.toolStatusLabel ? 6 : 0) +
            (item.isToolRunning ? 4 : 0);
        return clampHeight(height, CHAT_MIN_TOOL_ROW_HEIGHT_PX, 720);
    }

    private estimateReasoningMessageHeight(item: CodexGuiListItem): number {
        const wrappedLines = estimateWrappedLineCount(item.plainContent, 64);
        const height = CHAT_ROW_PADDING_PX + wrappedLines * 24;
        return clampHeight(height, CHAT_MIN_REASONING_ROW_HEIGHT_PX, 480);
    }

    private estimateStandardMessageHeight(item: CodexGuiListItem): number {
        const text = item.streamingPlain ? item.plainContent : stripHtmlTags(item.html);
        const wrappedLines = estimateWrappedLineCount(
            text,
            item.dataRole === "system" || item.dataRole === "reasoning" ? 58 : 64,
        );
        const preCount = countMatches(item.html, /<pre\b/gi);
        const blockquoteCount = countMatches(item.html, /<blockquote\b/gi);
        const listItemCount = countMatches(item.html, /<li\b/gi);
        const headingCount = countMatches(item.html, /<h[1-6]\b/gi);
        const tableCount = countMatches(item.html, /<table\b/gi);
        const height =
            CHAT_ROW_PADDING_PX +
            wrappedLines * 26 +
            preCount * 72 +
            blockquoteCount * 24 +
            listItemCount * 10 +
            headingCount * 20 +
            tableCount * 96 +
            (item.dataRole === "system" || item.dataRole === "reasoning" ? 12 : 0);
        return clampHeight(height, CHAT_MIN_STANDARD_ROW_HEIGHT_PX, 1400);
    }

    private pruneMeasuredHistoryItemHeights(): void {
        const activeKeys = new Set(this.historyListItems.map((item) => item.trackKey));
        for (const key of this.measuredHistoryItemHeights.keys()) {
            if (!activeKeys.has(key)) {
                this.measuredHistoryItemHeights.delete(key);
            }
        }
    }

    private scheduleHistoryMeasurement(): void {
        this.cancelHistoryMeasurement();
        this.zone.runOutsideAngular(() => {
            this.historyMeasurementFrameId = requestAnimationFrame(() => {
                this.historyMeasurementFrameId = undefined;
                this.measureRenderedHistoryRows();
            });
        });
    }

    private cancelHistoryMeasurement(): void {
        if (this.historyMeasurementFrameId === undefined) {
            return;
        }
        cancelAnimationFrame(this.historyMeasurementFrameId);
        this.historyMeasurementFrameId = undefined;
    }

    private measureRenderedHistoryRows(): void {
        const rowElements = Array.from(
            this.scrollHost?.querySelectorAll<HTMLElement>(
                ".messages-viewport .message-row[data-track-key]",
            ) ?? [],
        );
        if (rowElements.length === 0) {
            return;
        }

        let hasMeasurementsChanged = false;
        for (const row of rowElements) {
            const trackKey = row.dataset["trackKey"];
            if (!trackKey) {
                continue;
            }
            const measuredHeight = Math.ceil(row.offsetHeight);
            if (measuredHeight <= 0) {
                continue;
            }
            if (this.measuredHistoryItemHeights.get(trackKey) === measuredHeight) {
                continue;
            }
            this.measuredHistoryItemHeights.set(trackKey, measuredHeight);
            hasMeasurementsChanged = true;
        }

        if (!hasMeasurementsChanged) {
            return;
        }

        this.zone.run(() => {
            this.historyListItems = [...this.historyListItems];
            this.cdr.detectChanges();
            if (this.bottomPinController.isPinned) {
                this.flushPendingScrollToBottom();
            }
        });
    }

    private buildTypingIndicatorItems(): CodexGuiListItem[] {
        if (!this.shouldShowGlobalTypingIndicator(this.messages, this.isWorking)) {
            return [];
        }

        const label = this.globalTypingLabel(this.messages);
        return [
            {
                key: "typing-indicator",
                trackKey: "typing-indicator",
                rowRole: "assistant",
                dataRole: "assistant",
                dataStatus: "streaming",
                renderKind: "typing",
                html: "",
                plainContent: "",
                streamingPlain: false,
                toolRowsHtml: [],
                isToolRunning: false,
                toolStatusLabel: "",
                showStreamingIndicator: false,
                typingLabel: this.activityLabel.trim() || label,
                typingStartedAt: this.activityStartedAt,
                showTypingLabel: this.shouldShowGlobalTypingLabel(label),
                compactWithNext: false,
            },
        ];
    }

    private buildMessageItem(message: Message): CodexGuiListItem {
        const cacheKey = `${this.stripPathPrefix}\u0000${message.id}`;
        const sourceKey =
            `${message.role}\u0000${message.status}\u0000${message.content}`;
        const cached = this.messageItemCache.get(cacheKey);
        if (cached?.sourceKey === sourceKey) {
            return cached.item;
        }
        const revision = cached ? cached.revision + 1 : 0;

        const rendered = renderCodexGuiMessage(message, this.stripPathPrefix);
        const item: CodexGuiListItem = {
            key: message.id,
            trackKey: `${message.id}:${revision}`,
            rowRole: message.role === "user" ? "user" : "assistant",
            dataRole: message.role,
            dataStatus: message.status,
            renderKind: rendered.renderKind,
            html: rendered.html,
            plainContent: rendered.plainContent,
            streamingPlain: rendered.streamingPlain,
            toolRowsHtml: rendered.toolRowsHtml,
            isToolRunning: rendered.isToolRunning,
            toolStatusLabel: rendered.toolStatusLabel,
            showStreamingIndicator: rendered.showStreamingIndicator,
            typingLabel: "",
            typingStartedAt: null,
            showTypingLabel: false,
            compactWithNext: false,
        };
        this.messageItemCache.set(cacheKey, { sourceKey, revision, item });
        return item;
    }

    private shouldShowGlobalTypingIndicator(
        messages: readonly Message[],
        isWorking: boolean,
    ): boolean {
        return shouldShowGlobalTypingIndicator(messages, isWorking);
    }

    private globalTypingLabel(messages: readonly Message[]): string {
        return globalTypingLabel(messages);
    }

    private shouldShowGlobalTypingLabel(label: string): boolean {
        return (this.activityLabel.trim() || label).trim().length > 0;
    }

    private bindScrollHost(): void {
        this.removeScrollListener?.();
        this.disconnectResizeObserver();
        const scrollHost = this.scrollHost;
        if (!scrollHost) {
            this.removeScrollListener = undefined;
            return;
        }

        const onScroll = (): void => {
            const distanceFromBottom =
                scrollHost.scrollHeight -
                scrollHost.scrollTop -
                scrollHost.clientHeight;
            if (distanceFromBottom <= STICK_THRESHOLD_PX) {
                this.bottomPinController.pin();
                return;
            }
            this.bottomPinController.updateFromScrollHost(scrollHost);
        };

        this.zone.runOutsideAngular(() => {
            scrollHost.addEventListener("scroll", onScroll, {
                passive: true,
            });
        });
        this.bindResizeObserver(scrollHost);
        onScroll();
        this.removeScrollListener = () =>
            scrollHost.removeEventListener("scroll", onScroll);
    }

    private bindResizeObserver(scrollHost: HTMLElement): void {
        if (typeof ResizeObserver === "undefined") {
            return;
        }
        this.resizeObserver = new ResizeObserver(() => {
            if (!this.isActive || !this.bottomPinController.isPinned) {
                return;
            }
            this.zone.runOutsideAngular(() => {
                if (this.resizeSyncFrameId !== undefined) {
                    cancelAnimationFrame(this.resizeSyncFrameId);
                }
                this.resizeSyncFrameId = requestAnimationFrame(() => {
                    this.resizeSyncFrameId = undefined;
                    if (!this.isActive || !this.bottomPinController.isPinned) {
                        return;
                    }
                    this.requestBottomSync();
                });
            });
        });
        this.resizeObserver.observe(scrollHost);
    }

    private disconnectResizeObserver(): void {
        this.resizeObserver?.disconnect();
        this.resizeObserver = undefined;
        if (this.resizeSyncFrameId === undefined) {
            return;
        }
        cancelAnimationFrame(this.resizeSyncFrameId);
        this.resizeSyncFrameId = undefined;
    }

    private shouldUseBottomFollow(
        previousTailAnchorKey: string | undefined,
        previousHistoryLength: number,
    ): boolean {
        return (
            this.tailItems.length > 0 &&
            this.tailItems[0]?.key === previousTailAnchorKey &&
            this.historyListItems.length === previousHistoryLength
        );
    }

    private requestBottomSync(): void {
        this.bottomSyncController.request({
            scrollHost: this.scrollHost,
        });
    }

    private flushPendingScrollToBottom(): void {
        this.bottomSyncController.flush({
            scrollHost: this.scrollHost,
        });
    }

    private requestBottomFollow(): void {
        this.bottomFollowController.request(this.scrollHost, () =>
            this.requestBottomSync(),
        );
    }

    private scheduleActivationBottomSync(): void {
        this.bottomSyncController.scheduleActivationSync({
            isActive: () => this.isActive,
            requestBottomSync: () => {
                this.requestBottomSync();
            },
        });
    }

    private clearActivationBottomSync(): void {
        this.bottomSyncController.clearActivationSync();
    }

    private clearPendingBottomSnap(): void {
        this.bottomFollowController.clear();
        this.bottomSyncController.clear();
    }

    private pathFromAnchorHref(href: string): string | null {
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

    private toDisplayPath(path: string, prefix: string): string {
        const normalizedPath = path.replaceAll("\\", "/");
        if (!prefix) {
            return normalizedPath;
        }
        for (const candidate of this.matchablePathPrefixes(prefix)) {
            const normalizedPrefix = candidate.endsWith("/")
                ? candidate
                : `${candidate}/`;
            if (normalizedPath.startsWith(normalizedPrefix)) {
                return normalizedPath.slice(normalizedPrefix.length);
            }
        }
        return normalizedPath;
    }

    private normalizeDiffPath(path: string): string {
        const normalizedPath = path.replaceAll("\\", "/");
        const prefix = this.stripPathPrefix.trim();
        if (!prefix) {
            return normalizedPath;
        }
        for (const candidate of this.matchablePathPrefixes(prefix)) {
            const normalizedPrefix = candidate.endsWith("/")
                ? candidate
                : `${candidate}/`;
            if (normalizedPath.startsWith(normalizedPrefix)) {
                return normalizedPath.slice(normalizedPrefix.length);
            }
        }
        return normalizedPath;
    }

    private matchablePathPrefixes(prefix: string): string[] {
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
}
