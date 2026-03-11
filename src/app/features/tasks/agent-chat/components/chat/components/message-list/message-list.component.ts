import { CommonModule } from "@angular/common";
import {
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
import { Datasource, SizeStrategy, UiScrollModule } from "ngx-ui-scroll";
import { Message, ToolRow } from "../../../../models";
import { LoadingSpinnerComponent } from "../../../../../../../shared/components/loading-spinner/loading-spinner.component";
import {
    globalTypingLabel,
    renderAgentChatMessage,
    shouldShowGlobalTypingIndicator,
} from "./agent-chat-message-list-renderer";
import { TypingIndicatorComponent } from "../typing-indicator/typing-indicator.component";
import { ThrobberComponent } from "../throbber/throbber.component";
import { ReasoningMessageComponent } from "./components/reasoning-message/reasoning-message.component";
import { StandardMessageComponent } from "./components/standard-message/standard-message.component";
import { ToolMessageComponent } from "./components/tool-message/tool-message.component";
import { UserMessageComponent } from "./components/user-message/user-message.component";

type AgentChatListItem = {
    key: string;
    trackKey: string;
    rowRole: "user" | "assistant";
    dataRole: "user" | "assistant" | "system" | "reasoning";
    dataStatus: "streaming" | "complete" | "error";
    renderKind: "user" | "tool" | "standard" | "reasoning" | "typing";
    html: string;
    plainContent: string;
    streamingPlain: boolean;
    toolRows: ToolRow[];
    isToolRunning: boolean;
    toolStatusLabel: string;
    showStreamingIndicator: boolean;
    typingLabel: string;
    typingStartedAt: string | null;
    showTypingLabel: boolean;
    compactWithNext: boolean;
};

const STICK_THRESHOLD_PX = 64;
const CHAT_BUFFER_SIZE = 10;
const CHAT_PADDING = 0.5;
const CHAT_ACTIVATION_SYNC_DELAYS_MS = [0, 40, 120, 280];
const CHAT_RESUME_BOTTOM_SYNC_FRAMES = 4;
const CHAT_SETTLE_BOTTOM_SYNC_FRAMES = 1;

@Component({
    selector: "app-agent-chat-message-list",
    standalone: true,
    imports: [
        CommonModule,
        UiScrollModule,
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

    @ViewChild("scrollHost")
    set scrollHostRef(host: ElementRef<HTMLElement> | undefined) {
        this.scrollHost = host?.nativeElement;
        this.bindScrollHost();
        if (this.pinnedToBottom) {
            this.requestBottomSync(CHAT_SETTLE_BOTTOM_SYNC_FRAMES);
        }
    }

    @ViewChild("tailContent")
    set tailContentRef(host: ElementRef<HTMLElement> | undefined) {
        this.tailContent = host?.nativeElement;
        this.bindTailObserver();
    }

    readonly datasource = new Datasource<AgentChatListItem>({
        get: (index, count, success) => {
            success(this.getRenderedItems(index, count));
        },
        settings: {
            minIndex: 0,
            startIndex: 0,
            bufferSize: CHAT_BUFFER_SIZE,
            padding: CHAT_PADDING,
            sizeStrategy: SizeStrategy.Average,
        },
    });

    historyItems: AgentChatListItem[] = [];
    tailItems: AgentChatListItem[] = [];

    private appliedHistoryItems: AgentChatListItem[] = [];
    private scrollHost?: HTMLElement;
    private tailContent?: HTMLElement;
    private removeScrollListener?: () => void;
    private resizeObserver?: ResizeObserver;
    private tailResizeObserver?: ResizeObserver;
    private resizeSyncFrameId?: number;
    private tailResizeSyncFrameId?: number;
    private adapterCheckFrameId?: number;
    private adapterInitRetryFrameId?: number;
    private bottomSyncFrameId?: number;
    private bottomSyncFramesRemaining = 0;
    private pinnedToBottom = true;
    private activationTimeoutIds: number[] = [];
    private adapterWork = Promise.resolve();
    private destroyed = false;

    constructor(private readonly zone: NgZone) {}

    ngOnChanges(changes: SimpleChanges): void {
        const contentChanged =
            changes["messages"] ||
            changes["isWorking"] ||
            changes["activityLabel"] ||
            changes["activityStartedAt"] ||
            changes["stripPathPrefix"];
        const becameVisible =
            changes["showInitialLoading"] &&
            !this.showInitialLoading &&
            this.isActive;
        const becameActive = changes["isActive"]?.currentValue === true;

        if (contentChanged) {
            this.rebuildItems();
            this.scheduleHistorySync();
            if (this.pinnedToBottom && this.tailItems.length > 0) {
                this.requestBottomSync(CHAT_SETTLE_BOTTOM_SYNC_FRAMES);
            }
        }

        if (becameVisible || becameActive) {
            this.pinnedToBottom = true;
            this.scheduleHistorySync(true);
            this.scheduleActivationSync();
            this.requestBottomSync(CHAT_RESUME_BOTTOM_SYNC_FRAMES);
        } else if (changes["isActive"]) {
            this.clearActivationSync();
            this.cancelBottomSync();
        }
    }

    ngOnDestroy(): void {
        this.destroyed = true;
        this.removeScrollListener?.();
        this.disconnectResizeObserver();
        this.disconnectTailObserver();
        this.clearActivationSync();
        this.cancelAdapterCheck();
        this.cancelAdapterInitRetry();
        this.cancelBottomSync();
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

    trackByItem(_index: number, item: AgentChatListItem): string {
        return item.trackKey;
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

        this.historyItems = historyMessages.map((message) =>
            this.buildMessageItem(message),
        );
        this.tailItems = liveTailMessages.map((message) =>
            this.buildMessageItem(message),
        );
        const typingIndicator = this.buildTypingIndicatorItem();
        if (typingIndicator) {
            this.tailItems = [...this.tailItems, typingIndicator];
        }
        this.applyToolSpacingCompaction();
    }

    private buildTypingIndicatorItem(): AgentChatListItem | null {
        if (!this.shouldShowGlobalTypingIndicator(this.messages, this.isWorking)) {
            return null;
        }

        const label = this.globalTypingLabel(this.messages);
        return {
            key: "typing-indicator",
            trackKey: [
                "typing-indicator",
                this.activityLabel.trim() || label,
                this.activityStartedAt ?? "",
                this.isWorking ? "working" : "idle",
            ].join("\u0000"),
            rowRole: "assistant",
            dataRole: "assistant",
            dataStatus: "streaming",
            renderKind: "typing",
            html: "",
            plainContent: "",
            streamingPlain: false,
            toolRows: [],
            isToolRunning: false,
            toolStatusLabel: "",
            showStreamingIndicator: false,
            typingLabel: this.activityLabel.trim() || label,
            typingStartedAt: this.activityStartedAt,
            showTypingLabel: this.shouldShowGlobalTypingLabel(label),
            compactWithNext: false,
        };
    }

    private buildMessageItem(message: Message): AgentChatListItem {
        const rendered = renderAgentChatMessage(message, this.stripPathPrefix);
        return {
            key: message.id,
            trackKey: [
                message.id,
                message.role,
                message.status,
                message.content,
                JSON.stringify(message.presentation),
            ].join("\u0000"),
            rowRole: message.role === "user" ? "user" : "assistant",
            dataRole: message.role,
            dataStatus: message.status,
            renderKind: rendered.renderKind,
            html: rendered.html,
            plainContent: rendered.plainContent,
            streamingPlain: rendered.streamingPlain,
            toolRows: rendered.toolRows,
            isToolRunning: rendered.isToolRunning,
            toolStatusLabel: rendered.toolStatusLabel,
            showStreamingIndicator: rendered.showStreamingIndicator,
            typingLabel: "",
            typingStartedAt: null,
            showTypingLabel: false,
            compactWithNext: false,
        };
    }

    private applyToolSpacingCompaction(): void {
        for (let index = 0; index < this.historyItems.length; index += 1) {
            const current = this.historyItems[index];
            const next =
                this.historyItems[index + 1] ??
                (index === this.historyItems.length - 1
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

    private getRenderedItems(index: number, count: number): AgentChatListItem[] {
        if (count <= 0) {
            return [];
        }
        const startIndex = Math.max(0, index);
        if (startIndex >= this.appliedHistoryItems.length) {
            return [];
        }
        return this.appliedHistoryItems.slice(startIndex, startIndex + count);
    }

    private scheduleHistorySync(forceReload = false): void {
        const nextItems = [...this.historyItems];
        this.queueAdapterWork(async () => {
            if (this.destroyed) {
                return;
            }

            const previousItems = [...this.appliedHistoryItems];
            this.appliedHistoryItems = this.mergeAppliedItems(previousItems, nextItems);
            const applied = await this.applyAdapterChange(
                previousItems,
                nextItems,
                forceReload,
            );
            if (!applied) {
                this.appliedHistoryItems = previousItems;
            }
        });
    }

    private queueAdapterWork(task: () => Promise<void>): void {
        this.adapterWork = this.adapterWork
            .catch(() => undefined)
            .then(async () => {
                if (this.destroyed) {
                    return;
                }
                await task();
            });
    }

    private async applyAdapterChange(
        previousItems: readonly AgentChatListItem[],
        nextItems: readonly AgentChatListItem[],
        forceReload: boolean,
    ): Promise<boolean> {
        if (!this.isActive || this.showInitialLoading) {
            return false;
        }

        const adapter = this.datasource.adapter;
        if (!adapter.init) {
            this.scheduleAdapterInitRetry();
            return false;
        }

        if (nextItems.length === 0) {
            await adapter.reload(0);
            return true;
        }

        if (forceReload || previousItems.length === 0) {
            await this.reloadScroller(
                this.pinnedToBottom
                    ? this.getBottomStartIndex(nextItems.length)
                    : this.getAnchorIndex(),
                CHAT_RESUME_BOTTOM_SYNC_FRAMES,
            );
            return true;
        }

        if (this.haveSameKeys(previousItems, nextItems)) {
            await adapter.update({
                predicate: (item) => {
                    const replacement = nextItems[item.$index];
                    if (!replacement) {
                        return false;
                    }
                    return replacement.trackKey === item.data.trackKey
                        ? true
                        : [replacement];
                },
                fixRight: this.pinnedToBottom,
            });
            if (this.pinnedToBottom) {
                this.requestBottomSync(CHAT_SETTLE_BOTTOM_SYNC_FRAMES);
            }
            this.scheduleAdapterCheck();
            return true;
        }

        const commonPrefixLength = this.getCommonPrefixLength(previousItems, nextItems);
        if (
            commonPrefixLength === previousItems.length &&
            nextItems.length > previousItems.length
        ) {
            await adapter.append({
                items: nextItems.slice(previousItems.length),
                ...(this.pinnedToBottom ? { eof: true } : { virtualize: true }),
            });
            await adapter.check();
            if (this.pinnedToBottom) {
                this.requestBottomSync(CHAT_RESUME_BOTTOM_SYNC_FRAMES);
            }
            return true;
        }

        await this.reloadScroller(
            this.pinnedToBottom
                ? this.getBottomStartIndex(nextItems.length)
                : this.getAnchorIndex(),
            CHAT_RESUME_BOTTOM_SYNC_FRAMES,
        );
        return true;
    }

    private mergeAppliedItems(
        previousItems: readonly AgentChatListItem[],
        nextItems: readonly AgentChatListItem[],
    ): AgentChatListItem[] {
        if (previousItems.length === 0 || nextItems.length === 0) {
            return [...nextItems];
        }

        const previousByKey = new Map(
            previousItems.map((item) => [item.key, item] as const),
        );

        return nextItems.map((item) => {
            const previous = previousByKey.get(item.key);
            return previous && previous.trackKey === item.trackKey ? previous : item;
        });
    }

    private async reloadScroller(
        startIndex: number,
        bottomSyncFrames = CHAT_SETTLE_BOTTOM_SYNC_FRAMES,
    ): Promise<void> {
        if (this.appliedHistoryItems.length === 0) {
            await this.datasource.adapter.reload(0);
            return;
        }

        const clampedIndex = Math.max(
            0,
            Math.min(startIndex, this.appliedHistoryItems.length - 1),
        );
        await this.datasource.adapter.reload(clampedIndex);
        await this.datasource.adapter.check();
        if (this.pinnedToBottom) {
            this.requestBottomSync(bottomSyncFrames);
        }
    }

    private haveSameKeys(
        previousItems: readonly AgentChatListItem[],
        nextItems: readonly AgentChatListItem[],
    ): boolean {
        if (previousItems.length !== nextItems.length) {
            return false;
        }
        for (let index = 0; index < previousItems.length; index += 1) {
            if (previousItems[index].key !== nextItems[index].key) {
                return false;
            }
        }
        return true;
    }

    private getCommonPrefixLength(
        previousItems: readonly AgentChatListItem[],
        nextItems: readonly AgentChatListItem[],
    ): number {
        const maxLength = Math.min(previousItems.length, nextItems.length);
        let index = 0;
        while (index < maxLength && previousItems[index].key === nextItems[index].key) {
            index += 1;
        }
        return index;
    }

    private getBottomStartIndex(itemCount: number): number {
        return Math.max(0, itemCount - 1);
    }

    private getAnchorIndex(): number {
        return this.datasource.adapter.firstVisible?.$index ?? 0;
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
            this.pinnedToBottom = this.isNearBottom(scrollHost);
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
            if (!this.isActive || this.showInitialLoading) {
                return;
            }

            this.zone.runOutsideAngular(() => {
                if (this.resizeSyncFrameId !== undefined) {
                    cancelAnimationFrame(this.resizeSyncFrameId);
                }
                this.resizeSyncFrameId = requestAnimationFrame(() => {
                    this.resizeSyncFrameId = undefined;
                    this.queueAdapterWork(async () => {
                        if (!this.datasource.adapter.init) {
                            return;
                        }
                        await this.datasource.adapter.check();
                        if (this.pinnedToBottom && this.tailItems.length === 0) {
                            this.requestBottomSync(CHAT_SETTLE_BOTTOM_SYNC_FRAMES);
                        }
                    });
                });
            });
        });
        this.resizeObserver.observe(scrollHost);
    }

    private bindTailObserver(): void {
        this.disconnectTailObserver();
        if (typeof ResizeObserver === "undefined" || !this.tailContent) {
            return;
        }

        this.tailResizeObserver = new ResizeObserver(() => {
            if (!this.isActive || !this.pinnedToBottom) {
                return;
            }

            this.zone.runOutsideAngular(() => {
                if (this.tailResizeSyncFrameId !== undefined) {
                    cancelAnimationFrame(this.tailResizeSyncFrameId);
                }
                this.tailResizeSyncFrameId = requestAnimationFrame(() => {
                    this.tailResizeSyncFrameId = undefined;
                    if (!this.isActive || !this.pinnedToBottom) {
                        return;
                    }
                    this.requestBottomSync(CHAT_SETTLE_BOTTOM_SYNC_FRAMES);
                });
            });
        });
        this.tailResizeObserver.observe(this.tailContent);
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

    private disconnectTailObserver(): void {
        this.tailResizeObserver?.disconnect();
        this.tailResizeObserver = undefined;
        if (this.tailResizeSyncFrameId === undefined) {
            return;
        }
        cancelAnimationFrame(this.tailResizeSyncFrameId);
        this.tailResizeSyncFrameId = undefined;
    }

    private scheduleAdapterCheck(): void {
        if (this.adapterCheckFrameId !== undefined) {
            return;
        }
        this.zone.runOutsideAngular(() => {
            this.adapterCheckFrameId = requestAnimationFrame(() => {
                this.adapterCheckFrameId = undefined;
                this.queueAdapterWork(async () => {
                    if (!this.datasource.adapter.init) {
                        return;
                    }
                    await this.datasource.adapter.check();
                });
            });
        });
    }

    private scheduleAdapterInitRetry(): void {
        if (this.adapterInitRetryFrameId !== undefined) {
            return;
        }
        this.zone.runOutsideAngular(() => {
            this.adapterInitRetryFrameId = requestAnimationFrame(() => {
                this.adapterInitRetryFrameId = undefined;
                if (this.destroyed || !this.isActive || this.showInitialLoading) {
                    return;
                }
                this.scheduleHistorySync(true);
            });
        });
    }

    private cancelAdapterCheck(): void {
        if (this.adapterCheckFrameId === undefined) {
            return;
        }
        cancelAnimationFrame(this.adapterCheckFrameId);
        this.adapterCheckFrameId = undefined;
    }

    private cancelAdapterInitRetry(): void {
        if (this.adapterInitRetryFrameId === undefined) {
            return;
        }
        cancelAnimationFrame(this.adapterInitRetryFrameId);
        this.adapterInitRetryFrameId = undefined;
    }

    private requestBottomSync(frames = CHAT_SETTLE_BOTTOM_SYNC_FRAMES): void {
        this.scheduleBottomSync(frames);
    }

    private scheduleBottomSync(frames: number): void {
        if (!this.scrollHost) {
            return;
        }
        this.bottomSyncFramesRemaining = Math.max(
            this.bottomSyncFramesRemaining,
            frames,
        );
        if (this.bottomSyncFrameId !== undefined) {
            return;
        }
        this.zone.runOutsideAngular(() => {
            this.bottomSyncFrameId = requestAnimationFrame(() => {
                this.bottomSyncFrameId = undefined;
                const scrollHost = this.scrollHost;
                if (!scrollHost) {
                    this.bottomSyncFramesRemaining = 0;
                    return;
                }
                scrollHost.scrollTop = scrollHost.scrollHeight;
                this.pinnedToBottom = true;
                this.bottomSyncFramesRemaining = Math.max(
                    0,
                    this.bottomSyncFramesRemaining - 1,
                );
                if (this.bottomSyncFramesRemaining > 0) {
                    this.scheduleBottomSync(this.bottomSyncFramesRemaining);
                }
            });
        });
    }

    private cancelBottomSync(): void {
        this.bottomSyncFramesRemaining = 0;
        if (this.bottomSyncFrameId === undefined) {
            return;
        }
        cancelAnimationFrame(this.bottomSyncFrameId);
        this.bottomSyncFrameId = undefined;
    }

    private isNearBottom(scrollHost: HTMLElement): boolean {
        const distanceFromBottom =
            scrollHost.scrollHeight -
            scrollHost.scrollTop -
            scrollHost.clientHeight;
        return distanceFromBottom <= STICK_THRESHOLD_PX;
    }

    private scheduleActivationSync(): void {
        this.clearActivationSync();
        this.zone.runOutsideAngular(() => {
            this.activationTimeoutIds = CHAT_ACTIVATION_SYNC_DELAYS_MS.map((delayMs) =>
                window.setTimeout(() => {
                    if (
                        !this.isActive ||
                        this.showInitialLoading ||
                        (this.historyItems.length === 0 && this.tailItems.length === 0)
                    ) {
                        return;
                    }
                    if (this.pinnedToBottom) {
                        this.requestBottomSync(CHAT_RESUME_BOTTOM_SYNC_FRAMES);
                    }
                }, delayMs),
            );
        });
    }

    private clearActivationSync(): void {
        for (const timeoutId of this.activationTimeoutIds) {
            clearTimeout(timeoutId);
        }
        this.activationTimeoutIds = [];
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
