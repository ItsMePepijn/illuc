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
import { marked } from "marked";
import DOMPurify from "dompurify";
import { DomSanitizer, SafeHtml } from "@angular/platform-browser";
import { Subscription } from "rxjs";
import {
    DiffLineType,
    DiffMode,
    DiffPayload,
    ReviewComment,
    ReviewCommentStatus,
    ReviewStore,
} from "../../../task.models";
import { TaskStore } from "../../../task.store";
import { LauncherService } from "../../../../launcher/launcher.service";
import { TaskReviewService } from "../../task-review.service";
import {
    FileTreeComponent,
    FileTreeNode,
} from "../file-tree/file-tree.component";

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

marked.setOptions({
    breaks: true,
    gfm: true,
});

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

interface RenderedDiffLine {
    type: DiffLineType;
    html: SafeHtml;
}

type DiffRowKind = "header" | "line" | "spacer" | "thread";

interface RenderedDiffRow {
    kind: DiffRowKind;
    filePath: string;
    displayName?: string;
    status?: string;
    line?: RenderedDiffLine;
    lineNumberOld?: number | null;
    lineNumberNew?: number | null;
    threadTarget?: {
        filePath: string;
        lineNumberOld?: number | null;
        lineNumberNew?: number | null;
        lineType: DiffLineType;
    };
}

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
    imports: [CommonModule, FileTreeComponent, ScrollingModule, FormsModule],
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
    commentThreads = new Map<string, ReviewComment[]>();
    activeThreadKey: string | null = null;
    isSubmittingComment = false;
    userDisplayName = "User";
    readonly reviewStatusOptions = REVIEW_STATUS_OPTIONS;
    private readonly collapsedThreads = new Set<string>();
    private readonly commentDrafts = new Map<string, string>();
    private readonly commentBodyCache = new Map<string, SafeHtml>();
    private readonly commentStatusUpdating = new Set<string>();
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
            this.shouldAutoCollapseThreads = true;
            void this.refreshReviewStore();
        }
        if (changes["taskId"]) {
            void this.ensureUserDisplayName();
        }
    }

    ngOnDestroy(): void {
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
                const hasThread = (this.commentThreads.get(key)?.length ?? 0) > 0;
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
        return this.commentThreads.get(key)?.length ?? 0;
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

    isStatusUpdating(comment: ReviewComment): boolean {
        return this.commentStatusUpdating.has(comment.id);
    }

    async updateCommentStatus(
        comment: ReviewComment,
        status: ReviewCommentStatus,
    ): Promise<void> {
        if (!this.taskId || !this.worktreePath) {
            return;
        }
        const nextStatus = status ?? DEFAULT_REVIEW_STATUS;
        if (comment.status === nextStatus) {
            return;
        }
        const previousStatus = comment.status ?? DEFAULT_REVIEW_STATUS;
        this.updateLocalCommentStatus(comment.id, nextStatus);
        this.commentStatusUpdating.add(comment.id);
        this.cdr.detectChanges();
        try {
            const updated = await this.reviewService.updateCommentStatus({
                worktreePath: this.worktreePath,
                taskId: this.taskId,
                commentId: comment.id,
                status: nextStatus,
            });
            this.replaceLocalComment(comment.id, updated);
        } catch (error) {
            console.error("Failed to update review comment status", error);
            this.updateLocalCommentStatus(comment.id, previousStatus);
        } finally {
            this.commentStatusUpdating.delete(comment.id);
            this.cdr.detectChanges();
        }
    }

    toggleThread(row: RenderedDiffRow, event?: Event): void {
        event?.stopPropagation();
        const key = this.buildLineKey(row);
        if (!key) {
            return;
        }
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
                filePath: target.filePath,
                lineNumberOld: target.lineNumberOld ?? null,
                lineNumberNew: target.lineNumberNew ?? null,
                lineType: target.lineType,
                status: DEFAULT_REVIEW_STATUS,
                body: draft,
                author: "user",
                createdAt: new Date().toISOString(),
            };
            this.applyLocalComment(optimisticComment);
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
                this.replaceLocalComment(optimisticId, comment);
            } catch (error) {
                console.error("Failed to add review comment", error);
                this.removeLocalComment(optimisticId);
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
            filePath: row.filePath,
            lineNumberOld: row.lineNumberOld ?? null,
            lineNumberNew: row.lineNumberNew ?? null,
            lineType,
            status: DEFAULT_REVIEW_STATUS,
            body: draft,
            author: "user",
            createdAt: new Date().toISOString(),
        };
        this.applyLocalComment(optimisticComment);
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
            this.replaceLocalComment(optimisticId, comment);
        } catch (error) {
            console.error("Failed to add review comment", error);
            this.removeLocalComment(optimisticId);
        } finally {
            this.isSubmittingComment = false;
            this.cdr.detectChanges();
        }
    }

    displayNameFor(author: string): string {
        if (author === "user") {
            return this.userDisplayName || "User";
        }
        return author;
    }

    renderCommentBody(comment: ReviewComment): SafeHtml {
        const normalizedBody = comment.body.replace(/\s+$/u, "");
        const key = `${comment.id}:${normalizedBody}`;
        const cached = this.commentBodyCache.get(key);
        if (cached) {
            return cached;
        }
        const rawHtml = marked.parse(normalizedBody) as string;
        const sanitized = DOMPurify.sanitize(rawHtml, {
            USE_PROFILES: { html: true },
        });
        const safe = this.sanitizer.bypassSecurityTrustHtml(sanitized);
        this.commentBodyCache.set(key, safe);
        return safe;
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
        return this.commentThreads.get(key) ?? [];
    }


    private isCommentableType(type: DiffLineType): boolean {
        return type === "add" || type === "del" || type === "context";
    }

    private applyLocalComment(comment: ReviewComment): void {
        const store = this.reviewStore ?? { ...this.emptyReviewStore };
        const taskId = this.taskId ?? "";
        const entry =
            store.tasks[taskId] ?? {
                taskId,
                comments: [],
            };
        const updatedStore: ReviewStore = {
            ...store,
            tasks: {
                ...store.tasks,
                [taskId]: {
                    ...entry,
                    comments: [...entry.comments, comment],
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
    ): void {
        const store = this.reviewStore ?? { ...this.emptyReviewStore };
        const taskId = this.taskId ?? "";
        const entry =
            store.tasks[taskId] ?? {
                taskId,
                comments: [],
            };
        const comments = entry.comments ?? [];
        const index = comments.findIndex((item) => item.id === optimisticId);
        const nextComments =
            index >= 0
                ? comments.map((item, idx) =>
                      idx === index ? comment : item,
                  )
                : [...comments, comment];
        this.reviewStore = {
            ...store,
            tasks: {
                ...store.tasks,
                [taskId]: {
                    ...entry,
                    comments: nextComments,
                },
            },
        };
        this.rebuildThreads();
        this.refreshRenderedRows();
    }

    private removeLocalComment(optimisticId: string): void {
        const store = this.reviewStore ?? { ...this.emptyReviewStore };
        const taskId = this.taskId ?? "";
        const entry =
            store.tasks[taskId] ?? {
                taskId,
                comments: [],
            };
        const comments = entry.comments ?? [];
        if (!comments.length) {
            return;
        }
        const nextComments = comments.filter(
            (item) => item.id !== optimisticId,
        );
        this.reviewStore = {
            ...store,
            tasks: {
                ...store.tasks,
                [taskId]: {
                    ...entry,
                    comments: nextComments,
                },
            },
        };
        this.rebuildThreads();
        this.refreshRenderedRows();
    }

    private updateLocalCommentStatus(
        commentId: string,
        status: ReviewCommentStatus,
    ): void {
        const store = this.reviewStore ?? { ...this.emptyReviewStore };
        const taskId = this.taskId ?? "";
        const entry = store.tasks[taskId];
        if (!entry?.comments?.length) {
            return;
        }
        const nextComments = entry.comments.map((comment) =>
            comment.id === commentId ? { ...comment, status } : comment,
        );
        this.reviewStore = {
            ...store,
            tasks: {
                ...store.tasks,
                [taskId]: {
                    ...entry,
                    comments: nextComments,
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
        const threads = new Map<string, ReviewComment[]>();
        if (!this.taskId || !this.reviewStore) {
            this.commentThreads = threads;
            return;
        }
        const entry = this.reviewStore.tasks[this.taskId];
        if (!entry || !entry.comments?.length) {
            this.commentThreads = threads;
            return;
        }
        for (const comment of entry.comments) {
            if (comment.lineNumberOld == null && comment.lineNumberNew == null) {
                continue;
            }
            const key = this.buildLineKeyFromParts(
                comment.filePath,
                comment.lineNumberOld ?? null,
                comment.lineNumberNew ?? null,
            );
            const list = threads.get(key) ?? [];
            list.push(comment);
            threads.set(key, list);
        }
        this.commentThreads = threads;
        if (this.shouldAutoCollapseThreads) {
            for (const [key, comments] of threads) {
                if (this.shouldCollapseThread(comments)) {
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

    private shouldCollapseThread(comments: ReviewComment[]): boolean {
        if (!comments.length) {
            return false;
        }
        return comments.every((comment) => {
            const status = comment.status ?? DEFAULT_REVIEW_STATUS;
            return (
                status === "resolved" ||
                status === "closed" ||
                status === "wont-fix"
            );
        });
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
