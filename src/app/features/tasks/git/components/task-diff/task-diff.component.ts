import { CommonModule } from "@angular/common";
import {
    ChangeDetectionStrategy,
    ChangeDetectorRef,
    Component,
    Input,
    OnChanges,
    OnDestroy,
    SimpleChanges,
    ViewChild,
} from "@angular/core";
import { FormsModule } from "@angular/forms";
import {
    CdkVirtualScrollViewport,
    ScrollingModule,
    VIRTUAL_SCROLL_STRATEGY,
} from "@angular/cdk/scrolling";
import { AutoSizeVirtualScrollStrategy } from "@angular/cdk-experimental/scrolling";
import hljs from "highlight.js/lib/core";
import javascript from "highlight.js/lib/languages/javascript";
import json from "highlight.js/lib/languages/json";
import typescript from "highlight.js/lib/languages/typescript";
import xml from "highlight.js/lib/languages/xml";
import css from "highlight.js/lib/languages/css";
import scss from "highlight.js/lib/languages/scss";
import bash from "highlight.js/lib/languages/bash";
import python from "highlight.js/lib/languages/python";
import java from "highlight.js/lib/languages/java";
import go from "highlight.js/lib/languages/go";
import rust from "highlight.js/lib/languages/rust";
import yaml from "highlight.js/lib/languages/yaml";
import csharp from "highlight.js/lib/languages/csharp";
import c from "highlight.js/lib/languages/c";
import cpp from "highlight.js/lib/languages/cpp";
import markdown from "highlight.js/lib/languages/markdown";
import { DomSanitizer, SafeHtml } from "@angular/platform-browser";
import { Subscription } from "rxjs";
import {
    DiffLineType,
    DiffMode,
    DiffPayload,
} from "../../models";
import {
    ReviewComment,
    ReviewCommentStatus,
    ReviewThread,
    ReviewStore,
} from "../../../review/models";
import { TaskStore } from "../../../task.store";
import { LauncherService } from "../../../../launcher/launcher.service";
import { TaskReviewService } from "../../../review/task-review.service";
import {
    FileTreeComponent,
    FileTreeNode,
} from "../file-tree/file-tree.component";
import { TaskDiffThreadComponent } from "../../../review/components/task-diff-thread/task-diff-thread.component";
import { RenderedDiffRow } from "./task-diff.models";

hljs.registerLanguage("javascript", javascript);
hljs.registerLanguage("typescript", typescript);
hljs.registerLanguage("json", json);
hljs.registerLanguage("xml", xml);
hljs.registerLanguage("html", xml);
hljs.registerLanguage("css", css);
hljs.registerLanguage("scss", scss);
hljs.registerLanguage("bash", bash);
hljs.registerLanguage("shell", bash);
hljs.registerLanguage("python", python);
hljs.registerLanguage("java", java);
hljs.registerLanguage("go", go);
hljs.registerLanguage("rust", rust);
hljs.registerLanguage("yaml", yaml);
hljs.registerLanguage("csharp", csharp);
hljs.registerLanguage("c", c);
hljs.registerLanguage("cpp", cpp);
hljs.registerLanguage("markdown", markdown);

type ReviewStatusOption = {
    value: ReviewCommentStatus;
    label: string;
};

const REVIEW_STATUS_OPTIONS: ReadonlyArray<ReviewStatusOption> = [
    { value: "active", label: "Active" },
    { value: "pending", label: "Pending" },
    { value: "resolved", label: "Resolved" },
    { value: "wont-fix", label: "Won't Fix" },
    { value: "closed", label: "Closed" },
];

const DEFAULT_REVIEW_STATUS: ReviewCommentStatus = "active";
const THREAD_CLOSE_DELAY_MS = 120;
const THREAD_CLOSE_ANIMATION_MS = 180;
const THREAD_CLOSE_TOTAL_MS = THREAD_CLOSE_DELAY_MS + THREAD_CLOSE_ANIMATION_MS;

const DIFF_MIN_BUFFER_PX = 400;
const DIFF_MAX_BUFFER_PX = 800;

function diffScrollStrategyFactory() {
    return new AutoSizeVirtualScrollStrategy(
        DIFF_MIN_BUFFER_PX,
        DIFF_MAX_BUFFER_PX,
    );
}

@Component({
    selector: "app-task-diff",
    standalone: true,
    imports: [
        CommonModule,
        FileTreeComponent,
        ScrollingModule,
        FormsModule,
        TaskDiffThreadComponent,
    ],
    templateUrl: "./task-diff.component.html",
    styleUrl: "./task-diff.component.css",
    changeDetection: ChangeDetectionStrategy.OnPush,
    providers: [
        {
            provide: VIRTUAL_SCROLL_STRATEGY,
            useFactory: diffScrollStrategyFactory,
        },
    ],
})
export class TaskDiffComponent implements OnChanges, OnDestroy {
    @Input() taskId: string | null = null;
    @Input() baseBranch: string | null = null;
    @Input() worktreePath: string | null = null;
    @ViewChild(CdkVirtualScrollViewport)
    diffViewport?: CdkVirtualScrollViewport;

    diffPayload: DiffPayload | null = null;
    renderedRows: RenderedDiffRow[] = [];
    private rowIndexByPath = new Map<string, number>();
    fileTree: FileTreeNode[] = [];
    lastUpdated: Date | null = null;
    error: string | null = null;
    isLoading = false;
    hasLoaded = false;
    diffMode: DiffMode = "worktree";
    readonly rowHeight = 28;
    private diffSubscription?: Subscription;
    private diffWatchStop?: () => Promise<void>;
    private watchVersion = 0;
    private reviewVersion = 0;
    private readonly emptyReviewStore: ReviewStore = {
        version: 1,
        tasks: {},
    };

    reviewStore: ReviewStore | null = null;
    commentThreads = new Map<string, ReviewThread>();
    activeThreadKey: string | null = null;
    isSubmittingComment = false;
    userDisplayName = "User";
    readonly reviewStatusOptions = REVIEW_STATUS_OPTIONS;
    private readonly collapsedThreads = new Set<string>();
    private readonly commentDrafts = new Map<string, string>();
    readonly threadStatusUpdating = new Set<string>();
    readonly editingCommentIds = new Set<string>();
    readonly deletingCommentIds = new Set<string>();
    readonly closingThreadKeys = new Set<string>();
    private readonly closingThreadTimers = new Map<string, ReturnType<typeof setTimeout>>();
    private shouldAutoCollapseThreads = true;

    constructor(
        private readonly taskStore: TaskStore,
        private readonly sanitizer: DomSanitizer,
        private readonly cdr: ChangeDetectorRef,
        private readonly launcher: LauncherService,
        private readonly reviewService: TaskReviewService,
    ) {}

    ngOnChanges(changes: SimpleChanges): void {
        if (changes["taskId"]) {
            void this.restartDiffWatch();
        }
        if (changes["taskId"] || changes["worktreePath"]) {
            this.activeThreadKey = null;
            this.commentDrafts.clear();
            this.collapsedThreads.clear();
            this.threadStatusUpdating.clear();
            this.editingCommentIds.clear();
            this.deletingCommentIds.clear();
            this.shouldAutoCollapseThreads = true;
            void this.refreshReviewStore();
        }
        if (changes["taskId"]) {
            void this.ensureUserDisplayName();
        }
    }

    ngOnDestroy(): void {
        this.clearClosingThreadTimers();
        void this.stopDiffWatch();
    }

    setDiffMode(mode: DiffMode): void {
        if (this.diffMode === mode) {
            return;
        }
        this.diffMode = mode;
        this.diffPayload = null;
        this.renderedRows = [];
        this.rowIndexByPath = new Map();
        this.fileTree = [];
        this.error = null;
        this.isLoading = false;
        this.hasLoaded = false;
        void this.restartDiffWatch();
    }

    private async restartDiffWatch(): Promise<void> {
        const watchVersion = ++this.watchVersion;
        await this.stopDiffWatch();
        if (this.watchVersion !== watchVersion) {
            return;
        }
        this.diffPayload = null;
        this.renderedRows = [];
        this.rowIndexByPath = new Map();
        this.fileTree = [];
        this.error = null;
        this.isLoading = false;
        this.hasLoaded = false;
        if (!this.taskId) {
            return;
        }
        this.isLoading = true;
        this.hasLoaded = false;
        this.cdr.detectChanges();
        const handle = this.taskStore.watchDiff(this.taskId, this.diffMode);
        this.diffWatchStop = handle.stop;
        this.diffSubscription = handle.state$.subscribe((state) => {
            if (this.watchVersion !== watchVersion) {
                return;
            }
            if (state.payload && state.payload.taskId !== this.taskId) {
                return;
            }
            this.diffPayload = state.payload;
            this.rebuildThreads();
            this.renderedRows = state.payload
                ? this.buildRenderedRows(state.payload)
                : [];
            this.fileTree = state.payload
                ? this.buildFileTree(state.payload.files)
                : [];
            this.lastUpdated = state.lastUpdated;
            this.error = state.error;
            this.isLoading = state.isLoading;
            this.hasLoaded = state.hasLoaded;
            this.cdr.detectChanges();
            this.diffViewport?.checkViewportSize();
        });
    }

    private async stopDiffWatch(): Promise<void> {
        this.diffSubscription?.unsubscribe();
        this.diffSubscription = undefined;
        if (this.diffWatchStop) {
            await this.diffWatchStop();
            this.diffWatchStop = undefined;
        }
    }


    scrollToFile(path: string): void {
        const index = this.rowIndexByPath.get(path);
        if (index === undefined) {
            return;
        }
        this.diffViewport?.scrollToIndex(index, "smooth");
    }

    async openFileInVsCode(filePath: string, event?: Event): Promise<void> {
        event?.stopPropagation();
        const resolved = this.resolveFilePath(filePath);
        if (!resolved) {
            return;
        }
        try {
            await this.launcher.openFileInVsCode(resolved, 1, 1);
        } catch (error) {
            console.error("Failed to open file in VS Code", error);
        }
    }

    trackByRow(index: number): number {
        return index;
    }

    private buildRenderedRows(payload: DiffPayload): RenderedDiffRow[] {
        const highlightEnabled = true;
        const rows: RenderedDiffRow[] = [];
        const indexByPath = new Map<string, number>();
        let firstFile = true;
        for (const file of payload.files) {
            if (!firstFile) {
                rows.push({
                    kind: "spacer",
                    filePath: file.path,
                });
            }
            firstFile = false;
            if (!indexByPath.has(file.path)) {
                indexByPath.set(file.path, rows.length);
            }
            rows.push({
                kind: "header",
                filePath: file.path,
                status: file.status,
            });
            for (const line of file.lines) {
                if (line.type === "meta") {
                    continue;
                }
                const lineRow: RenderedDiffRow = {
                    kind: "line",
                    filePath: file.path,
                    lineNumberOld: line.lineNumberOld ?? null,
                    lineNumberNew: line.lineNumberNew ?? null,
                    line: {
                        type: line.type,
                        html: this.renderLine(
                            line.content,
                            file.path,
                            line.type,
                            highlightEnabled,
                        ),
                    },
                };
                rows.push(lineRow);
                const key = this.buildLineKeyFromParts(
                    file.path,
                    line.lineNumberOld ?? null,
                    line.lineNumberNew ?? null,
                );
                const hasThread =
                    (this.commentThreads.get(key)?.comments?.length ?? 0) > 0;
                const isActive = this.activeThreadKey === key;
                if ((hasThread && !this.collapsedThreads.has(key)) || isActive) {
                    rows.push({
                        kind: "thread",
                        filePath: file.path,
                        threadTarget: {
                            filePath: file.path,
                            lineNumberOld: line.lineNumberOld ?? null,
                            lineNumberNew: line.lineNumberNew ?? null,
                            lineType: line.type,
                        },
                    });
                }
            }
        }
        this.rowIndexByPath = indexByPath;
        return rows;
    }

    private resolveFilePath(filePath: string): string | null {
        if (!this.worktreePath) {
            return null;
        }
        const base = this.worktreePath.replace(/[\\/]+$/, "");
        const separator = base.includes("\\") ? "\\" : "/";
        const normalized =
            separator === "\\" ? filePath.replace(/\//g, "\\") : filePath;
        return `${base}${separator}${normalized}`;
    }


    private buildFileTree(files: DiffPayload["files"]): FileTreeNode[] {
        type BuildNode = {
            name: string;
            path: string;
            isFile: boolean;
            status?: string;
            children: Map<string, BuildNode>;
        };
        const root: BuildNode = {
            name: "",
            path: "",
            isFile: false,
            children: new Map(),
        };
        for (const file of files) {
            const parts = file.path.split("/").filter(Boolean);
            let current = root;
            let currentPath = "";
            for (let index = 0; index < parts.length; index += 1) {
                const part = parts[index];
                currentPath = currentPath ? `${currentPath}/${part}` : part;
                let child = current.children.get(part);
                if (!child) {
                    child = {
                        name: part,
                        path: currentPath,
                        isFile: false,
                        children: new Map(),
                    };
                    current.children.set(part, child);
                }
                if (index === parts.length - 1) {
                    child.isFile = true;
                    child.status = file.status;
                }
                current = child;
            }
        }
        const compressNode = (node: BuildNode): BuildNode => {
            if (node.isFile) {
                return node;
            }
            const children = Array.from(node.children.values()).map(
                compressNode,
            );
            node.children = new Map(
                children.map((child) => [child.name, child]),
            );
            let current = node;
            while (!current.isFile && current.children.size === 1) {
                const onlyChild = Array.from(current.children.values())[0];
                if (onlyChild.isFile) {
                    break;
                }
                current = {
                    name: current.name
                        ? `${current.name}/${onlyChild.name}`
                        : onlyChild.name,
                    path: onlyChild.path,
                    isFile: false,
                    children: onlyChild.children,
                };
            }
            return current;
        };

        const toArray = (node: BuildNode, depth: number): FileTreeNode[] => {
            const children = Array.from(node.children.values()).sort((a, b) => {
                if (a.isFile !== b.isFile) {
                    return a.isFile ? 1 : -1;
                }
                return a.name.localeCompare(b.name);
            });
            return children.map((child) => ({
                name: child.name,
                path: child.path,
                depth,
                isFile: child.isFile,
                status: child.status,
                children: toArray(child, depth + 1),
            }));
        };
        const compressedRoot = compressNode(root);
        return toArray(compressedRoot, 0);
    }

    isCommentableRow(row: RenderedDiffRow): boolean {
        if (row.kind !== "line" || !row.line) {
            return false;
        }
        return this.isCommentableType(row.line.type);
    }

    commentCount(row: RenderedDiffRow): number {
        const key = this.buildLineKey(row);
        if (!key) {
            return 0;
        }
        return this.commentThreads.get(key)?.comments?.length ?? 0;
    }

    hasComments(row: RenderedDiffRow): boolean {
        return this.commentCount(row) > 0;
    }

    isThreadCollapsed(row: RenderedDiffRow): boolean {
        const key = this.buildLineKey(row);
        if (!key) {
            return false;
        }
        return this.collapsedThreads.has(key);
    }

    isThreadOpen(row: RenderedDiffRow): boolean {
        const key = this.buildLineKey(row);
        return !!key && this.activeThreadKey === key;
    }

    isThreadClosing(row: RenderedDiffRow): boolean {
        if (row.kind !== "thread" || !row.threadTarget) {
            return false;
        }
        const key = this.buildLineKeyFromParts(
            row.threadTarget.filePath,
            row.threadTarget.lineNumberOld ?? null,
            row.threadTarget.lineNumberNew ?? null,
        );
        return this.closingThreadKeys.has(key);
    }

    commentsFor(row: RenderedDiffRow): ReviewComment[] {
        const target =
            row.threadTarget ??
            (row.lineNumberOld !== undefined || row.lineNumberNew !== undefined
                ? {
                      filePath: row.filePath,
                      lineNumberOld: row.lineNumberOld ?? null,
                      lineNumberNew: row.lineNumberNew ?? null,
                  }
                : null);
        if (!target) {
            return [];
        }
        return this.commentsForTarget(
            target.filePath,
            target.lineNumberOld ?? null,
            target.lineNumberNew ?? null,
        );
    }

    async updateThreadStatus(
        row: RenderedDiffRow,
        status: ReviewCommentStatus,
    ): Promise<void> {
        if (!this.taskId || !this.worktreePath) {
            return;
        }
        const key = this.draftKeyForRow(row);
        if (!key || row.kind !== "thread" || !row.threadTarget) {
            return;
        }
        const nextStatus = status ?? DEFAULT_REVIEW_STATUS;
        const previousStatus = this.threadStatusForKey(key);
        const previousWasCollapsed = this.collapsedThreads.has(key);
        const previousActiveThreadKey = this.activeThreadKey;
        if (previousStatus === nextStatus) {
            return;
        }
        this.updateLocalThreadStatus(key, nextStatus);
        this.syncThreadCollapseStateForStatus(key, nextStatus);
        this.threadStatusUpdating.add(key);
        this.cdr.detectChanges();
        try {
            await this.reviewService.updateThreadStatus({
                worktreePath: this.worktreePath,
                taskId: this.taskId,
                filePath: row.threadTarget.filePath,
                lineNumberOld: row.threadTarget.lineNumberOld ?? null,
                lineNumberNew: row.threadTarget.lineNumberNew ?? null,
                status: nextStatus,
            });
        } catch (error) {
            console.error("Failed to update review thread status", error);
            this.updateLocalThreadStatus(key, previousStatus);
            this.restoreThreadCollapseState(
                key,
                previousWasCollapsed,
                previousActiveThreadKey,
            );
        } finally {
            this.threadStatusUpdating.delete(key);
            this.cdr.detectChanges();
        }
    }

    toggleThread(row: RenderedDiffRow, event?: Event): void {
        event?.stopPropagation();
        const key = this.buildLineKey(row);
        if (!key) {
            return;
        }
        this.cancelScheduledThreadCollapse(key);
        if (this.activeThreadKey === key) {
            this.activeThreadKey = null;
        } else {
            this.activeThreadKey = key;
        }
        this.renderedRows = this.diffPayload
            ? this.buildRenderedRows(this.diffPayload)
            : [];
        this.cdr.detectChanges();
    }

    toggleThreadCollapsed(row: RenderedDiffRow, event?: Event): void {
        event?.stopPropagation();
        if (row.kind !== "thread" || !row.threadTarget) {
            return;
        }
        const key = this.buildLineKeyFromParts(
            row.threadTarget.filePath,
            row.threadTarget.lineNumberOld ?? null,
            row.threadTarget.lineNumberNew ?? null,
        );
        this.cancelScheduledThreadCollapse(key);
        if (this.collapsedThreads.has(key)) {
            this.collapsedThreads.delete(key);
        } else {
            this.collapsedThreads.add(key);
        }
        if (this.activeThreadKey === key) {
            this.activeThreadKey = null;
        }
        this.renderedRows = this.diffPayload
            ? this.buildRenderedRows(this.diffPayload)
            : [];
        this.cdr.detectChanges();
    }

    openCollapsedThread(row: RenderedDiffRow, event?: Event): void {
        event?.stopPropagation();
        const key = this.buildLineKey(row);
        if (!key) {
            return;
        }
        this.cancelScheduledThreadCollapse(key);
        if (this.collapsedThreads.has(key)) {
            this.collapsedThreads.delete(key);
        }
        this.activeThreadKey = key;
        this.renderedRows = this.diffPayload
            ? this.buildRenderedRows(this.diffPayload)
            : [];
        this.cdr.detectChanges();
    }

    async submitComment(row: RenderedDiffRow, event?: Event): Promise<void> {
        event?.preventDefault();
        event?.stopPropagation();
        if (row.kind === "thread") {
            const target = row.threadTarget;
            if (!target) {
                return;
            }
            if (!this.taskId || !this.worktreePath) {
                return;
            }
            const draft = this.getDraft(row).trim();
            if (!draft) {
                return;
            }
            const optimisticId = this.buildOptimisticId();
            const optimisticComment: ReviewComment = {
                id: optimisticId,
                body: draft,
                author: "user",
                createdAt: new Date().toISOString(),
            };
            this.applyLocalComment(optimisticComment, {
                filePath: target.filePath,
                lineNumberOld: target.lineNumberOld ?? null,
                lineNumberNew: target.lineNumberNew ?? null,
                lineType: target.lineType,
            });
            this.clearDraft(row);
            this.activeThreadKey = this.buildLineKeyFromParts(
                target.filePath,
                target.lineNumberOld ?? null,
                target.lineNumberNew ?? null,
            );
            this.cdr.detectChanges();
            this.isSubmittingComment = true;
            this.cdr.detectChanges();
            try {
                const comment = await this.reviewService.addComment({
                    worktreePath: this.worktreePath,
                    taskId: this.taskId,
                    filePath: target.filePath,
                    lineNumberOld: target.lineNumberOld ?? null,
                    lineNumberNew: target.lineNumberNew ?? null,
                    lineType: target.lineType,
                    body: draft,
                });
                this.replaceLocalComment(optimisticId, comment, {
                    filePath: target.filePath,
                    lineNumberOld: target.lineNumberOld ?? null,
                    lineNumberNew: target.lineNumberNew ?? null,
                    lineType: target.lineType,
                });
            } catch (error) {
                console.error("Failed to add review comment", error);
                this.removeLocalComment(optimisticId, {
                    filePath: target.filePath,
                    lineNumberOld: target.lineNumberOld ?? null,
                    lineNumberNew: target.lineNumberNew ?? null,
                });
            } finally {
                this.isSubmittingComment = false;
                this.cdr.detectChanges();
            }
            return;
        }
        if (!this.isCommentableRow(row)) {
            return;
        }
        if (!this.taskId || !this.worktreePath) {
            return;
        }
        const draft = this.getDraft(row).trim();
        if (!draft) {
            return;
        }
        if (row.lineNumberOld == null && row.lineNumberNew == null) {
            return;
        }
        const lineType = row.line?.type;
        if (!lineType) {
            return;
        }
        const optimisticId = this.buildOptimisticId();
        const optimisticComment: ReviewComment = {
            id: optimisticId,
            body: draft,
            author: "user",
            createdAt: new Date().toISOString(),
        };
        this.applyLocalComment(optimisticComment, {
            filePath: row.filePath,
            lineNumberOld: row.lineNumberOld ?? null,
            lineNumberNew: row.lineNumberNew ?? null,
            lineType,
        });
        this.clearDraft(row);
        this.activeThreadKey = this.buildLineKey(row);
        this.cdr.detectChanges();
        this.isSubmittingComment = true;
        this.cdr.detectChanges();
        try {
            const comment = await this.reviewService.addComment({
                worktreePath: this.worktreePath,
                taskId: this.taskId,
                filePath: row.filePath,
                lineNumberOld: row.lineNumberOld ?? null,
                lineNumberNew: row.lineNumberNew ?? null,
                lineType,
                body: draft,
            });
            this.replaceLocalComment(optimisticId, comment, {
                filePath: row.filePath,
                lineNumberOld: row.lineNumberOld ?? null,
                lineNumberNew: row.lineNumberNew ?? null,
                lineType,
            });
        } catch (error) {
            console.error("Failed to add review comment", error);
            this.removeLocalComment(optimisticId, {
                filePath: row.filePath,
                lineNumberOld: row.lineNumberOld ?? null,
                lineNumberNew: row.lineNumberNew ?? null,
            });
        } finally {
            this.isSubmittingComment = false;
            this.cdr.detectChanges();
        }
    }

    async editComment(
        row: RenderedDiffRow,
        event: { comment: ReviewComment; body: string },
    ): Promise<void> {
        if (!this.taskId || !this.worktreePath || !row.threadTarget) {
            return;
        }
        const nextBody = event.body.trim();
        if (!nextBody) {
            return;
        }
        const commentId = event.comment.id;
        const threadKey = this.buildLineKeyFromParts(
            row.threadTarget.filePath,
            row.threadTarget.lineNumberOld ?? null,
            row.threadTarget.lineNumberNew ?? null,
        );
        const previousBody = event.comment.body;
        if (previousBody === nextBody) {
            return;
        }
        this.updateLocalCommentBody(threadKey, commentId, nextBody);
        this.editingCommentIds.add(commentId);
        this.cdr.detectChanges();
        try {
            const updated = await this.reviewService.editComment({
                worktreePath: this.worktreePath,
                taskId: this.taskId,
                filePath: row.threadTarget.filePath,
                lineNumberOld: row.threadTarget.lineNumberOld ?? null,
                lineNumberNew: row.threadTarget.lineNumberNew ?? null,
                commentId,
                body: nextBody,
            });
            this.replaceCommentById(threadKey, commentId, updated);
        } catch (error) {
            console.error("Failed to edit review comment", error);
            this.updateLocalCommentBody(threadKey, commentId, previousBody);
        } finally {
            this.editingCommentIds.delete(commentId);
            this.cdr.detectChanges();
        }
    }

    async deleteComment(row: RenderedDiffRow, comment: ReviewComment): Promise<void> {
        if (!this.taskId || !this.worktreePath || !row.threadTarget) {
            return;
        }
        const commentId = comment.id;
        const threadKey = this.buildLineKeyFromParts(
            row.threadTarget.filePath,
            row.threadTarget.lineNumberOld ?? null,
            row.threadTarget.lineNumberNew ?? null,
        );
        const removed = this.removeCommentById(threadKey, commentId);
        if (!removed) {
            return;
        }
        this.deletingCommentIds.add(commentId);
        this.cdr.detectChanges();
        try {
            await this.reviewService.deleteComment({
                worktreePath: this.worktreePath,
                taskId: this.taskId,
                filePath: row.threadTarget.filePath,
                lineNumberOld: row.threadTarget.lineNumberOld ?? null,
                lineNumberNew: row.threadTarget.lineNumberNew ?? null,
                commentId,
            });
        } catch (error) {
            console.error("Failed to delete review comment", error);
            this.restoreComment(
                threadKey,
                comment,
                row.threadTarget.filePath,
                row.threadTarget.lineNumberOld ?? null,
                row.threadTarget.lineNumberNew ?? null,
                row.threadTarget.lineType,
            );
        } finally {
            this.deletingCommentIds.delete(commentId);
            this.cdr.detectChanges();
        }
    }

    canSubmitComment(row: RenderedDiffRow): boolean {
        return (
            !this.isSubmittingComment &&
            this.getDraft(row).trim().length > 0
        );
    }

    getDraft(row: RenderedDiffRow): string {
        const key = this.draftKeyForRow(row);
        if (!key) {
            return "";
        }
        return this.commentDrafts.get(key) ?? "";
    }

    setDraft(row: RenderedDiffRow, value: string): void {
        const key = this.draftKeyForRow(row);
        if (!key) {
            return;
        }
        this.commentDrafts.set(key, value);
        this.cdr.detectChanges();
    }

    clearDraft(row: RenderedDiffRow): void {
        const key = this.draftKeyForRow(row);
        if (!key) {
            return;
        }
        this.commentDrafts.delete(key);
        this.cdr.detectChanges();
    }

    private draftKeyForRow(row: RenderedDiffRow): string | null {
        if (row.threadTarget) {
            return this.buildLineKeyFromParts(
                row.threadTarget.filePath,
                row.threadTarget.lineNumberOld ?? null,
                row.threadTarget.lineNumberNew ?? null,
            );
        }
        if (row.lineNumberOld == null && row.lineNumberNew == null) {
            return null;
        }
        return this.buildLineKeyFromParts(
            row.filePath,
            row.lineNumberOld ?? null,
            row.lineNumberNew ?? null,
        );
    }

    private buildLineKey(row: RenderedDiffRow): string | null {
        if (
            row.kind !== "line" ||
            (row.lineNumberOld == null && row.lineNumberNew == null)
        ) {
            return null;
        }
        return this.buildLineKeyFromParts(
            row.filePath,
            row.lineNumberOld ?? null,
            row.lineNumberNew ?? null,
        );
    }

    private buildLineKeyFromParts(
        filePath: string,
        lineNumberOld: number | null,
        lineNumberNew: number | null,
    ): string {
        const oldKey = lineNumberOld ?? "x";
        const newKey = lineNumberNew ?? "x";
        return `${filePath}::${oldKey}::${newKey}`;
    }

    private commentsForTarget(
        filePath: string,
        lineNumberOld: number | null,
        lineNumberNew: number | null,
    ): ReviewComment[] {
        const key = this.buildLineKeyFromParts(
            filePath,
            lineNumberOld,
            lineNumberNew,
        );
        return this.commentThreads.get(key)?.comments ?? [];
    }


    private isCommentableType(type: DiffLineType): boolean {
        return type === "add" || type === "del" || type === "context";
    }

    private applyLocalComment(
        comment: ReviewComment,
        target: {
            filePath: string;
            lineNumberOld: number | null;
            lineNumberNew: number | null;
            lineType: DiffLineType;
        },
    ): void {
        const store = this.reviewStore ?? { ...this.emptyReviewStore };
        const taskId = this.taskId ?? "";
        const entry =
            store.tasks[taskId] ?? {
                taskId,
                threads: [],
            };
        const threadKey = this.buildLineKeyFromParts(
            target.filePath,
            target.lineNumberOld,
            target.lineNumberNew,
        );
        const nextThreads = entry.threads.some(
            (thread) => this.threadKeyForThread(thread) === threadKey,
        )
            ? entry.threads.map((thread) =>
                  this.threadKeyForThread(thread) === threadKey
                      ? {
                            ...thread,
                            comments: [...(thread.comments ?? []), comment],
                        }
                      : thread,
              )
            : [
                  ...entry.threads,
                  {
                      filePath: target.filePath,
                      lineNumberOld: target.lineNumberOld,
                      lineNumberNew: target.lineNumberNew,
                      lineType: target.lineType,
                      status: DEFAULT_REVIEW_STATUS,
                      comments: [comment],
                  },
              ];
        const updatedStore: ReviewStore = {
            ...store,
            tasks: {
                ...store.tasks,
                [taskId]: {
                    ...entry,
                    threads: nextThreads,
                },
            },
        };
        this.reviewStore = updatedStore;
        this.rebuildThreads();
        this.refreshRenderedRows();
    }

    private replaceLocalComment(
        optimisticId: string,
        comment: ReviewComment,
        target: {
            filePath: string;
            lineNumberOld: number | null;
            lineNumberNew: number | null;
            lineType: DiffLineType;
        },
    ): void {
        const store = this.reviewStore ?? { ...this.emptyReviewStore };
        const taskId = this.taskId ?? "";
        const entry =
            store.tasks[taskId] ?? {
                taskId,
                threads: [],
            };
        const threadKey = this.buildLineKeyFromParts(
            target.filePath,
            target.lineNumberOld,
            target.lineNumberNew,
        );
        const nextThreads = entry.threads.some(
            (thread) => this.threadKeyForThread(thread) === threadKey,
        )
            ? entry.threads.map((thread) => {
                  if (this.threadKeyForThread(thread) !== threadKey) {
                      return thread;
                  }
                  const comments = thread.comments ?? [];
                  const index = comments.findIndex((item) => item.id === optimisticId);
                  const nextComments =
                      index >= 0
                          ? comments.map((item, idx) =>
                                idx === index ? comment : item,
                            )
                          : [...comments, comment];
                  return { ...thread, comments: nextComments };
              })
            : [
                  ...entry.threads,
                  {
                      filePath: target.filePath,
                      lineNumberOld: target.lineNumberOld,
                      lineNumberNew: target.lineNumberNew,
                      lineType: target.lineType,
                      status: DEFAULT_REVIEW_STATUS,
                      comments: [comment],
                  },
              ];
        this.reviewStore = {
            ...store,
            tasks: {
                ...store.tasks,
                [taskId]: {
                    ...entry,
                    threads: nextThreads,
                },
            },
        };
        this.rebuildThreads();
        this.refreshRenderedRows();
    }

    private removeLocalComment(
        optimisticId: string,
        target: {
            filePath: string;
            lineNumberOld: number | null;
            lineNumberNew: number | null;
        },
    ): void {
        const store = this.reviewStore ?? { ...this.emptyReviewStore };
        const taskId = this.taskId ?? "";
        const entry =
            store.tasks[taskId] ?? {
                taskId,
                threads: [],
            };
        if (!entry.threads.length) {
            return;
        }
        const threadKey = this.buildLineKeyFromParts(
            target.filePath,
            target.lineNumberOld,
            target.lineNumberNew,
        );
        const nextThreads = entry.threads
            .map((thread) => {
                if (this.threadKeyForThread(thread) !== threadKey) {
                    return thread;
                }
                const nextComments = (thread.comments ?? []).filter(
                    (item) => item.id !== optimisticId,
                );
                return { ...thread, comments: nextComments };
            })
            .filter((thread) => (thread.comments?.length ?? 0) > 0);
        this.reviewStore = {
            ...store,
            tasks: {
                ...store.tasks,
                [taskId]: {
                    ...entry,
                    threads: nextThreads,
                },
            },
        };
        this.rebuildThreads();
        this.refreshRenderedRows();
    }

    private updateLocalThreadStatus(
        threadKey: string,
        status: ReviewCommentStatus,
    ): void {
        const store = this.reviewStore ?? { ...this.emptyReviewStore };
        const taskId = this.taskId ?? "";
        const entry = store.tasks[taskId];
        if (!entry) {
            return;
        }
        const nextThreads = entry.threads.map((thread) =>
            this.threadKeyForThread(thread) === threadKey
                ? { ...thread, status }
                : thread,
        );
        this.reviewStore = {
            ...store,
            tasks: {
                ...store.tasks,
                [taskId]: {
                    ...entry,
                    threads: nextThreads,
                },
            },
        };
        this.rebuildThreads();
        this.refreshRenderedRows();
    }

    private updateLocalCommentBody(
        threadKey: string,
        commentId: string,
        body: string,
    ): void {
        const store = this.reviewStore ?? { ...this.emptyReviewStore };
        const taskId = this.taskId ?? "";
        const entry = store.tasks[taskId];
        if (!entry) {
            return;
        }
        const nextThreads = entry.threads.map((thread) =>
            this.threadKeyForThread(thread) !== threadKey
                ? thread
                : {
                      ...thread,
                      comments: (thread.comments ?? []).map((comment) =>
                          comment.id === commentId ? { ...comment, body } : comment,
                      ),
                  },
        );
        this.reviewStore = {
            ...store,
            tasks: {
                ...store.tasks,
                [taskId]: {
                    ...entry,
                    threads: nextThreads,
                },
            },
        };
        this.rebuildThreads();
        this.refreshRenderedRows();
    }

    private replaceCommentById(
        threadKey: string,
        commentId: string,
        updated: ReviewComment,
    ): void {
        const store = this.reviewStore ?? { ...this.emptyReviewStore };
        const taskId = this.taskId ?? "";
        const entry = store.tasks[taskId];
        if (!entry) {
            return;
        }
        const nextThreads = entry.threads.map((thread) =>
            this.threadKeyForThread(thread) !== threadKey
                ? thread
                : {
                      ...thread,
                      comments: (thread.comments ?? []).map((comment) =>
                          comment.id === commentId ? updated : comment,
                      ),
                  },
        );
        this.reviewStore = {
            ...store,
            tasks: {
                ...store.tasks,
                [taskId]: {
                    ...entry,
                    threads: nextThreads,
                },
            },
        };
        this.rebuildThreads();
        this.refreshRenderedRows();
    }

    private removeCommentById(threadKey: string, commentId: string): boolean {
        const store = this.reviewStore ?? { ...this.emptyReviewStore };
        const taskId = this.taskId ?? "";
        const entry = store.tasks[taskId];
        if (!entry) {
            return false;
        }
        let removed = false;
        const nextThreads = entry.threads
            .map((thread) => {
                if (this.threadKeyForThread(thread) !== threadKey) {
                    return thread;
                }
                const nextComments = (thread.comments ?? []).filter((comment) => {
                    const keep = comment.id !== commentId;
                    if (!keep) {
                        removed = true;
                    }
                    return keep;
                });
                return { ...thread, comments: nextComments };
            })
            .filter((thread) => (thread.comments?.length ?? 0) > 0);
        if (!removed) {
            return false;
        }
        this.reviewStore = {
            ...store,
            tasks: {
                ...store.tasks,
                [taskId]: {
                    ...entry,
                    threads: nextThreads,
                },
            },
        };
        this.rebuildThreads();
        this.refreshRenderedRows();
        return true;
    }

    private restoreComment(
        threadKey: string,
        comment: ReviewComment,
        filePath: string,
        lineNumberOld: number | null,
        lineNumberNew: number | null,
        lineType: DiffLineType,
    ): void {
        const store = this.reviewStore ?? { ...this.emptyReviewStore };
        const taskId = this.taskId ?? "";
        const entry = store.tasks[taskId];
        if (!entry) {
            return;
        }
        const nextThreads = entry.threads.some(
            (thread) => this.threadKeyForThread(thread) === threadKey,
        )
            ? entry.threads.map((thread) =>
                  this.threadKeyForThread(thread) !== threadKey
                      ? thread
                      : {
                            ...thread,
                            comments: [...(thread.comments ?? []), comment],
                        },
              )
            : [
                  ...entry.threads,
                  {
                      filePath,
                      lineNumberOld,
                      lineNumberNew,
                      lineType,
                      status: DEFAULT_REVIEW_STATUS,
                      comments: [comment],
                  },
              ];
        this.reviewStore = {
            ...store,
            tasks: {
                ...store.tasks,
                [taskId]: {
                    ...entry,
                    threads: nextThreads,
                },
            },
        };
        this.rebuildThreads();
        this.refreshRenderedRows();
    }

    private buildOptimisticId(): string {
        return `optimistic-${Date.now()}-${Math.random()
            .toString(36)
            .slice(2, 8)}`;
    }

    private async refreshReviewStore(): Promise<void> {
        const worktreePath = this.worktreePath?.trim();
        if (!worktreePath) {
            this.reviewStore = null;
            this.commentThreads = new Map();
            this.cdr.detectChanges();
            return;
        }
        const version = ++this.reviewVersion;
        try {
            const store = await this.reviewService.loadStore(worktreePath);
            if (this.reviewVersion !== version) {
                return;
            }
            this.reviewStore = store;
            this.rebuildThreads();
            this.cdr.detectChanges();
            this.refreshRenderedRows();
        } catch (error) {
            console.error("Failed to load review data", error);
            this.reviewStore = { ...this.emptyReviewStore };
            this.commentThreads = new Map();
            this.cdr.detectChanges();
        }
    }

    private rebuildThreads(): void {
        const threads = new Map<string, ReviewThread>();
        if (!this.taskId || !this.reviewStore) {
            this.commentThreads = threads;
            return;
        }
        const entry = this.reviewStore.tasks[this.taskId];
        if (!entry || !entry.threads?.length) {
            this.commentThreads = threads;
            return;
        }
        for (const thread of entry.threads) {
            const key = this.threadKeyForThread(thread);
            threads.set(key, thread);
        }
        this.commentThreads = threads;
        if (this.shouldAutoCollapseThreads) {
            for (const key of threads.keys()) {
                if (this.shouldCollapseThread(key)) {
                    this.collapsedThreads.add(key);
                }
            }
            this.shouldAutoCollapseThreads = false;
        }
        for (const key of Array.from(this.collapsedThreads)) {
            if (!threads.has(key)) {
                this.collapsedThreads.delete(key);
            }
        }
    }

    private shouldCollapseThread(threadKey: string): boolean {
        const status = this.threadStatusForKey(threadKey);
        return this.isCollapsingStatus(status);
    }

    threadStatusForRow(row: RenderedDiffRow): ReviewCommentStatus {
        const key = this.draftKeyForRow(row);
        if (!key) {
            return DEFAULT_REVIEW_STATUS;
        }
        return this.threadStatusForKey(key);
    }

    isThreadStatusUpdating(row: RenderedDiffRow): boolean {
        const key = this.draftKeyForRow(row);
        return !!key && this.threadStatusUpdating.has(key);
    }

    private threadStatusForKey(threadKey: string): ReviewCommentStatus {
        return this.commentThreads.get(threadKey)?.status ?? DEFAULT_REVIEW_STATUS;
    }

    private isCollapsingStatus(status: ReviewCommentStatus): boolean {
        return (
            status === "resolved" ||
            status === "closed" ||
            status === "wont-fix"
        );
    }

    private syncThreadCollapseStateForStatus(
        threadKey: string,
        status: ReviewCommentStatus,
    ): void {
        if (this.isCollapsingStatus(status)) {
            this.scheduleThreadCollapse(threadKey);
            return;
        }
        this.cancelScheduledThreadCollapse(threadKey);
    }

    private restoreThreadCollapseState(
        threadKey: string,
        wasCollapsed: boolean,
        previousActiveThreadKey: string | null,
    ): void {
        this.cancelScheduledThreadCollapse(threadKey);
        if (wasCollapsed) {
            this.collapsedThreads.add(threadKey);
        } else {
            this.collapsedThreads.delete(threadKey);
        }
        this.activeThreadKey = previousActiveThreadKey;
        this.refreshRenderedRows();
    }

    private scheduleThreadCollapse(threadKey: string): void {
        if (this.collapsedThreads.has(threadKey)) {
            return;
        }
        if (this.closingThreadKeys.has(threadKey)) {
            return;
        }
        this.closingThreadKeys.add(threadKey);
        this.refreshRenderedRows();
        const timer = setTimeout(() => {
            this.closingThreadTimers.delete(threadKey);
            this.closingThreadKeys.delete(threadKey);
            this.collapsedThreads.add(threadKey);
            if (this.activeThreadKey === threadKey) {
                this.activeThreadKey = null;
            }
            this.refreshRenderedRows();
        }, THREAD_CLOSE_TOTAL_MS);
        this.closingThreadTimers.set(threadKey, timer);
    }

    private cancelScheduledThreadCollapse(threadKey: string): void {
        const timer = this.closingThreadTimers.get(threadKey);
        if (timer) {
            clearTimeout(timer);
            this.closingThreadTimers.delete(threadKey);
        }
        if (this.closingThreadKeys.delete(threadKey)) {
            this.refreshRenderedRows();
        }
    }

    private clearClosingThreadTimers(): void {
        for (const timer of this.closingThreadTimers.values()) {
            clearTimeout(timer);
        }
        this.closingThreadTimers.clear();
        this.closingThreadKeys.clear();
    }

    private threadKeyForThread(thread: ReviewThread): string {
        return this.buildLineKeyFromParts(
            thread.filePath,
            thread.lineNumberOld ?? null,
            thread.lineNumberNew ?? null,
        );
    }

    private refreshRenderedRows(): void {
        if (!this.diffPayload) {
            return;
        }
        this.renderedRows = this.buildRenderedRows(this.diffPayload);
        this.cdr.detectChanges();
        this.diffViewport?.checkViewportSize();
    }

    private async ensureUserDisplayName(): Promise<void> {
        try {
            const displayName = await this.reviewService.getUserDisplayName();
            if (displayName?.trim()) {
                this.userDisplayName = displayName.trim();
                this.cdr.detectChanges();
            }
        } catch (error) {
            console.error("Failed to resolve user display name", error);
        }
    }

    private renderLine(
        line: string,
        filePath: string,
        type: DiffLineType,
        highlightEnabled: boolean,
    ): SafeHtml {
        if (type === "add" || type === "del" || type === "context") {
            const highlighted = highlightEnabled
                ? this.highlightContent(line, filePath)
                : this.escapeHtml(line);
            return this.sanitizer.bypassSecurityTrustHtml(
                `<span class="diff-code">${highlighted}</span>`,
            );
        }
        return this.sanitizer.bypassSecurityTrustHtml(
            `<span class="diff-code">${this.escapeHtml(line)}</span>`,
        );
    }

    private highlightContent(content: string, filePath: string): string {
        const language = this.detectLanguage(filePath);
        if (language && hljs.getLanguage(language)) {
            try {
                return hljs.highlight(content, { language }).value;
            } catch {
                return this.escapeHtml(content);
            }
        }
        return this.escapeHtml(content);
    }

    private detectLanguage(path: string): string | null {
        const ext = path.split(".").pop()?.toLowerCase();
        if (!ext) {
            return null;
        }
        const mapping: Record<string, string> = {
            ts: "typescript",
            tsx: "typescript",
            js: "javascript",
            jsx: "javascript",
            json: "json",
            html: "html",
            htm: "html",
            css: "css",
            scss: "scss",
            sass: "scss",
            md: "markdown",
            xml: "xml",
            yml: "yaml",
            yaml: "yaml",
            sh: "bash",
            bash: "bash",
            py: "python",
            java: "java",
            go: "go",
            rs: "rust",
            cs: "csharp",
            c: "c",
            h: "c",
            cpp: "cpp",
            hpp: "cpp",
        };
        return mapping[ext] ?? null;
    }

    private escapeHtml(value: string): string {
        return value
            .replace(/&/g, "&amp;")
            .replace(/</g, "&lt;")
            .replace(/>/g, "&gt;");
    }
}
