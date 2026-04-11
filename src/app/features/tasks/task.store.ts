import { Injectable, NgZone, OnDestroy, computed, signal } from "@angular/core";
import { type Event as TauriEvent, type UnlistenFn } from "@tauri-apps/api/event";
import { Observable, Subject } from "rxjs";
import { AgentKind, AgentKindAvailability, BaseRepoInfo, TaskSummary } from "./models";
import { DiffMode, DiffPayload } from "./git/models";
import {
    TerminalKind,
    TerminalExitEvent,
    TerminalOutputEvent,
} from "./terminal/models";
import { TaskGitService } from "./git/task-git.service";
import { TERMINAL_SCROLLBACK } from "./terminal/terminal.constants";
import { tauriInvoke, tauriListen } from "../../shared/tauri/tauri-zone";
import { AgentChatStore } from "./agent-chat/agent-chat.store";
import { readIllucHmrState } from "../../shared/hmr/hmr-state";

@Injectable({
    providedIn: "root",
})
export class TaskStore implements OnDestroy {
    private readonly maxTerminalLines = TERMINAL_SCROLLBACK;
    private readonly tasksSignal = signal<TaskSummary[]>([]);
    private readonly agentKindsSignal = signal<AgentKindAvailability[] | null>(null);
    private readonly baseRepoSignal = signal<BaseRepoInfo | null>(null);
    private readonly selectedTaskIdSignal = signal<string | null>(null);
    private readonly branchOptionsSignal = signal<string[]>([]);
    private readonly viewModeSignal = signal<"task" | "home">("task");
    private readonly lastEditorByTaskId = new Map<string, string>();
    private readonly terminalBuffers = new Map<string, string>();
    private readonly terminalStreams = new Map<string, Subject<string>>();
    private readonly terminalSizes = new Map<
        string,
        { cols: number; rows: number }
    >();
    private readonly terminalLastResizeSent = new Map<
        string,
        { cols: number; rows: number }
    >();
    private lastTerminalSize: { cols: number; rows: number } | null = null;
    private readonly worktreeTerminalBuffers = new Map<string, string>();
    private readonly worktreeTerminalStreams = new Map<
        string,
        Subject<string>
    >();
    private readonly worktreeTerminalSizes = new Map<
        string,
        { cols: number; rows: number }
    >();
    private readonly worktreeTerminalLastResizeSent = new Map<
        string,
        { cols: number; rows: number }
    >();
    private lastWorktreeTerminalSize: { cols: number; rows: number } | null =
        null;
    private readonly worktreeTerminalOpenState = new Map<string, boolean>();
    private readonly diffJumpStreams = new Map<string, Subject<string>>();
    private readonly unlistenFns: UnlistenFn[] = [];
    private readonly unloadHandler = () => this.teardown();
    private agentKindsLoadPromise: Promise<AgentKindAvailability[]> | null = null;

    private readonly diffRefreshDelayMs = 250;

    readonly tasks = this.tasksSignal.asReadonly();
    readonly agentKinds = this.agentKindsSignal.asReadonly();
    readonly baseRepo = this.baseRepoSignal.asReadonly();
    readonly selectedTaskId = this.selectedTaskIdSignal.asReadonly();
    readonly branchOptions = this.branchOptionsSignal.asReadonly();
    readonly viewMode = this.viewModeSignal.asReadonly();
    readonly selectedTask = computed(() => {
        const id = this.selectedTaskIdSignal();
        if (!id) {
            return null;
        }
        return this.tasksSignal().find((wf) => wf.taskId === id) ?? null;
    });

    constructor(
        private readonly zone: NgZone,
        private readonly taskGit: TaskGitService,
        private readonly agentChatStore: AgentChatStore,
    ) {
        const snapshot = readIllucHmrState()?.taskStore;
        if (snapshot) {
            this.restoreDevState(snapshot);
        }
        void this.preloadAgentKinds();
        this.registerEventListeners();
        window.addEventListener("unload", this.unloadHandler);
    }

    ngOnDestroy(): void {
        window.removeEventListener("unload", this.unloadHandler);
        this.teardown();
    }

    snapshotDevState(): TaskStoreDevState {
        return {
            baseRepo: this.baseRepoSignal(),
            tasks: [...this.tasksSignal()],
            selectedTaskId: this.selectedTaskIdSignal(),
            branchOptions: [...this.branchOptionsSignal()],
            viewMode: this.viewModeSignal(),
            lastEditorByTaskId: Object.fromEntries(this.lastEditorByTaskId),
            terminalBuffers: Object.fromEntries(this.terminalBuffers),
            worktreeTerminalBuffers: Object.fromEntries(
                this.worktreeTerminalBuffers,
            ),
            worktreeTerminalOpenState: Object.fromEntries(
                this.worktreeTerminalOpenState,
            ),
            terminalSizes: Object.fromEntries(this.terminalSizes),
            worktreeTerminalSizes: Object.fromEntries(
                this.worktreeTerminalSizes,
            ),
            lastTerminalSize: this.lastTerminalSize,
            lastWorktreeTerminalSize: this.lastWorktreeTerminalSize,
        };
    }

    restoreDevState(snapshot: TaskStoreDevState): void {
        this.baseRepoSignal.set(snapshot.baseRepo ?? null);
        this.tasksSignal.set(snapshot.tasks ?? []);
        this.selectedTaskIdSignal.set(snapshot.selectedTaskId ?? null);
        this.branchOptionsSignal.set(snapshot.branchOptions ?? []);
        this.viewModeSignal.set(snapshot.viewMode ?? "task");
        this.restoreMap(this.lastEditorByTaskId, snapshot.lastEditorByTaskId);
        this.restoreMap(this.terminalBuffers, snapshot.terminalBuffers);
        this.restoreMap(
            this.worktreeTerminalBuffers,
            snapshot.worktreeTerminalBuffers,
        );
        this.restoreMap(
            this.worktreeTerminalOpenState,
            snapshot.worktreeTerminalOpenState,
        );
        this.restoreMap(this.terminalSizes, snapshot.terminalSizes);
        this.restoreMap(
            this.worktreeTerminalSizes,
            snapshot.worktreeTerminalSizes,
        );
        this.lastTerminalSize = snapshot.lastTerminalSize ?? null;
        this.lastWorktreeTerminalSize = snapshot.lastWorktreeTerminalSize ?? null;
    }

    async selectBaseRepo(path: string): Promise<BaseRepoInfo> {
        const repo = await tauriInvoke<BaseRepoInfo>(this.zone, "select_base_repo", { path });
        const normalized: BaseRepoInfo = {
            ...repo,
            path: repo.canonicalPath,
        };
        this.baseRepoSignal.set(normalized);
        this.tasksSignal.set([]);
        this.viewModeSignal.set("home");
        this.selectedTaskIdSignal.set(null);
        this.branchOptionsSignal.set([]);
        this.terminalBuffers.clear();
        this.terminalStreams.clear();
        this.terminalLastResizeSent.clear();
        this.worktreeTerminalBuffers.clear();
        this.worktreeTerminalStreams.clear();
        this.worktreeTerminalLastResizeSent.clear();
        this.worktreeTerminalOpenState.clear();
        this.lastEditorByTaskId.clear();
        this.agentChatStore.clearAll();
        await this.loadExistingTasks(normalized.path);
        await this.loadBranches(normalized.path);
        return normalized;
    }

    async createTask(
        branchName: string,
        displayTitle: string,
        baseBranch?: string | null,
    ): Promise<TaskSummary> {
        const repo = this.baseRepoSignal();
        if (!repo) {
            throw new Error("Select a base repository before creating tasks.");
        }
        const baseRef = baseBranch?.trim() || repo.currentBranch || repo.head;
        const summary = await tauriInvoke<TaskSummary>(this.zone, "task_create", {
            req: {
                baseRepoPath: repo.path,
                baseRef,
                taskTitle: displayTitle.trim() || undefined,
                branchName: branchName.trim(),
            },
        });
        this.upsertTask(summary);
        this.selectedTaskIdSignal.set(summary.taskId);
        return summary;
    }

    async startTask(taskId: string, agent?: AgentKind): Promise<TaskSummary> {
        if (taskId) {
            this.agentChatStore.resetTask(taskId);
        }
        const size = this.terminalSizes.get(taskId) ?? this.lastTerminalSize;
        const summary = await tauriInvoke<TaskSummary>(this.zone, "task_start", {
            req: {
                taskId,
                cols: size?.cols,
                rows: size?.rows,
                agent,
            },
        });
        this.upsertTask(summary);
        return summary;
    }

    async preloadAgentKinds(): Promise<AgentKindAvailability[]> {
        const cached = this.agentKindsSignal();
        if (cached) {
            return cached;
        }
        if (this.agentKindsLoadPromise) {
            return this.agentKindsLoadPromise;
        }
        this.agentKindsLoadPromise = tauriInvoke<AgentKindAvailability[]>(
            this.zone,
            "task_agent_kinds_list",
        )
            .then((agentKinds) => {
                this.agentKindsSignal.set(agentKinds);
                return agentKinds;
            })
            .finally(() => {
                this.agentKindsLoadPromise = null;
            });
        return this.agentKindsLoadPromise;
    }

    async stopTask(taskId: string): Promise<TaskSummary> {
        const summary = await tauriInvoke<TaskSummary>(this.zone, "task_stop", {
            req: { taskId },
        });
        this.upsertTask(summary);
        return summary;
    }

    getTask(taskId: string): TaskSummary | null {
        if (!taskId) {
            return null;
        }
        return this.tasksSignal().find((task) => task.taskId === taskId) ?? null;
    }

    getLastEditorId(taskId: string): string | null {
        return this.lastEditorByTaskId.get(taskId) ?? null;
    }

    rememberEditorForTask(taskId: string, editorId: string): void {
        if (!taskId || !editorId) {
            return;
        }
        this.lastEditorByTaskId.set(taskId, editorId);
    }

    async discardTask(taskId: string): Promise<void> {
        await tauriInvoke<void>(this.zone, "task_discard", { req: { taskId } });
        this.removeTask(taskId);
    }

    async writeToAgentTui(taskId: string, data: string): Promise<void> {
        await tauriInvoke<void>(this.zone, "task_agent_tui_write", {
            req: { taskId, data },
        });
    }

    async resizeAgentTui(
        taskId: string,
        cols: number,
        rows: number,
    ): Promise<void> {
        const previous = this.terminalLastResizeSent.get(taskId);
        if (previous && previous.cols === cols && previous.rows === rows) {
            return;
        }
        await tauriInvoke<void>(this.zone, "task_agent_tui_resize", {
            req: { taskId, cols, rows },
        });
        this.terminalLastResizeSent.set(taskId, { cols, rows });
    }

    async startTerminal(taskId: string, kind: TerminalKind): Promise<void> {
        if (kind !== "worktree") {
            return;
        }
        const size =
            this.worktreeTerminalSizes.get(taskId) ??
            this.lastWorktreeTerminalSize;
        await tauriInvoke<void>(this.zone, "task_terminal_start", {
            req: {
                taskId,
                kind,
                cols: size?.cols,
                rows: size?.rows,
            },
        });
    }

    async writeToTerminal(
        taskId: string,
        data: string,
        kind: TerminalKind,
    ): Promise<void> {
        await tauriInvoke<void>(this.zone, "task_terminal_write", {
            req: { taskId, kind, data },
        });
    }

    async resizeTerminal(
        taskId: string,
        cols: number,
        rows: number,
        kind: TerminalKind,
    ): Promise<void> {
        const sent = this.selectLastResizeSent(kind);
        const previous = sent.get(taskId);
        if (previous && previous.cols === cols && previous.rows === rows) {
            return;
        }
        await tauriInvoke<void>(this.zone, "task_terminal_resize", {
            req: {
                taskId,
                kind,
                cols,
                rows,
            },
        });
        sent.set(taskId, { cols, rows });
    }

    async getDiff(
        taskId: string,
        ignoreWhitespace = false,
        mode: DiffMode = "worktree",
    ): Promise<DiffPayload> {
        return tauriInvoke<DiffPayload>(this.zone, "task_git_diff_get", {
            req: {
                taskId,
                ignoreWhitespace,
                mode,
            },
        });
    }

    async hasUncommittedChanges(taskId: string): Promise<boolean> {
        return tauriInvoke<boolean>(this.zone, "task_git_has_changes", {
            req: { taskId },
        });
    }

    async startDiffWatch(taskId: string): Promise<void> {
        await tauriInvoke<void>(this.zone, "task_git_diff_watch_start", { req: { taskId } });
    }

    async stopDiffWatch(taskId: string): Promise<void> {
        await tauriInvoke<void>(this.zone, "task_git_diff_watch_stop", { req: { taskId } });
    }

    watchDiff(taskId: string, mode: DiffMode): DiffWatchHandle {
        const watcher = new DiffWatcher({
            taskId,
            mode,
            refreshDelayMs: this.diffRefreshDelayMs,
            getDiff: (id, diffMode) => this.getDiff(id, false, diffMode),
            startDiffWatch: (id) => this.startDiffWatch(id),
            stopDiffWatch: (id) => this.stopDiffWatch(id),
            listen: <T>(
                event: string,
                handler: (event: TauriEvent<T>) => void,
            ) => tauriListen<T>(this.zone, event, handler),
            zone: this.zone,
        });
        void watcher.start();
        return {
            state$: watcher.state$,
            stop: () => watcher.stop(),
        };
    }

    async commitTask(
        taskId: string,
        message: string,
        stageAll = true,
    ): Promise<void> {
        await tauriInvoke<void>(this.zone, "task_git_commit", {
            req: {
                taskId,
                message,
                stageAll,
            },
        });
    }

    async pushTask(
        taskId: string,
        remote = "origin",
        branch?: string,
        setUpstream = true,
    ): Promise<void> {
        await tauriInvoke<void>(this.zone, "task_git_push", {
            req: {
                taskId,
                remote,
                branch,
                setUpstream,
            },
        });
    }

    async mergeTask(
        taskId: string,
        targetBranch: string,
        pushMainAfterMerge = false,
    ): Promise<void> {
        await tauriInvoke<void>(this.zone, "task_git_merge", {
            req: {
                taskId,
                targetBranch,
                pushMainAfterMerge,
            },
        });
    }

    selectTask(taskId: string | null): void {
        if (taskId) {
            this.viewModeSignal.set("task");
            this.selectedTaskIdSignal.set(taskId);
            return;
        }
        this.selectHome();
    }

    selectHome(): void {
        this.viewModeSignal.set("home");
        this.selectedTaskIdSignal.set(null);
    }

    branches(): string[] {
        return this.branchOptionsSignal();
    }

    defaultBaseBranch(): string | null {
        return this.baseRepoSignal()?.currentBranch ?? null;
    }

    getTerminalBuffer(taskId: string, kind: TerminalKind): string {
        const buffer = this.selectTerminalBuffer(kind);
        return buffer.get(taskId) ?? "";
    }

    clearTerminalBuffer(taskId: string, kind: TerminalKind): void {
        if (!taskId) {
            return;
        }
        const buffer = this.selectTerminalBuffer(kind);
        buffer.delete(taskId);
    }

    terminalOutput$(taskId: string, kind: TerminalKind): Observable<string> {
        const stream = this.ensureTerminalStream(taskId, kind);
        return stream.asObservable();
    }

    diffJump$(taskId: string): Observable<string> {
        return this.ensureDiffJumpStream(taskId).asObservable();
    }

    requestDiffJump(taskId: string, filePath: string): void {
        const normalizedPath = filePath.trim();
        if (!taskId || !normalizedPath) {
            return;
        }
        this.ensureDiffJumpStream(taskId).next(normalizedPath);
    }

    recordTerminalSize(
        taskId: string,
        cols: number,
        rows: number,
        kind: TerminalKind,
    ): void {
        if (cols <= 0 || rows <= 0) {
            return;
        }
        if (kind === "worktree") {
            this.lastWorktreeTerminalSize = { cols, rows };
            if (taskId) {
                this.worktreeTerminalSizes.set(taskId, { cols, rows });
            }
            return;
        }
        this.lastTerminalSize = { cols, rows };
        if (taskId) {
            this.terminalSizes.set(taskId, { cols, rows });
        }
    }

    isWorktreeTerminalOpen(taskId: string): boolean {
        return this.worktreeTerminalOpenState.get(taskId) ?? false;
    }

    setWorktreeTerminalOpen(taskId: string, isOpen: boolean): void {
        if (!taskId) {
            return;
        }
        this.worktreeTerminalOpenState.set(taskId, isOpen);
    }

    private registerEventListeners(): void {
        void tauriListen<TaskSummary>(this.zone, "task_status_changed", (event) => {
            this.upsertTask(event.payload);
        }).then((unlisten) => this.unlistenFns.push(unlisten));

        void tauriListen<TerminalOutputEvent>(this.zone, "task_terminal_output", (event) => {
            this.pushTerminalOutput(
                event.payload.taskId,
                event.payload.data,
                event.payload.kind,
            );
        }).then((unlisten) => this.unlistenFns.push(unlisten));

        void tauriListen<TerminalExitEvent>(this.zone, "task_terminal_exit", (event) => {
            console.info(
                `Terminal ${event.payload.kind} for ${event.payload.taskId} exited with code ${event.payload.exitCode}`,
            );
        }).then((unlisten) => this.unlistenFns.push(unlisten));
    }

    private upsertTask(summary: TaskSummary): void {
        this.tasksSignal.update((items) => {
            const existingIndex = items.findIndex(
                (item) => item.taskId === summary.taskId,
            );
            if (existingIndex >= 0) {
                const copy = [...items];
                copy[existingIndex] = summary;
                return copy;
            }
            return [...items, summary].sort((a, b) =>
                a.createdAt.localeCompare(b.createdAt),
            );
        });
        if (!this.selectedTaskIdSignal() && this.viewModeSignal() !== "home") {
            this.selectedTaskIdSignal.set(summary.taskId);
        }
    }

    private removeTask(taskId: string): void {
        let remaining: TaskSummary[] = [];
        this.tasksSignal.update((items) => {
            remaining = items.filter((item) => item.taskId !== taskId);
            return remaining;
        });
        if (this.selectedTaskIdSignal() === taskId) {
            this.selectedTaskIdSignal.set(
                this.viewModeSignal() === "task"
                    ? remaining[0]?.taskId ?? null
                    : null,
            );
        }
        this.terminalBuffers.delete(taskId);
        this.terminalStreams.delete(taskId);
        this.terminalLastResizeSent.delete(taskId);
        this.worktreeTerminalBuffers.delete(taskId);
        this.worktreeTerminalStreams.delete(taskId);
        this.worktreeTerminalLastResizeSent.delete(taskId);
        this.worktreeTerminalOpenState.delete(taskId);
        this.lastEditorByTaskId.delete(taskId);
        this.agentChatStore.removeTask(taskId);
        this.diffJumpStreams.delete(taskId);
    }

    private pushTerminalOutput(
        taskId: string,
        chunk: string,
        kind: TerminalKind,
    ): void {
        const buffer = this.selectTerminalBuffer(kind);
        const stream = this.ensureTerminalStream(taskId, kind);
        const current = buffer.get(taskId) ?? "";
        buffer.set(taskId, this.trimTerminalBuffer(current + chunk));
        stream.next(chunk);
    }

    private trimTerminalBuffer(value: string): string {
        const lines = value.split("\n");
        if (lines.length <= this.maxTerminalLines) {
            return value;
        }
        return lines.slice(-this.maxTerminalLines).join("\n");
    }

    private async loadExistingTasks(baseRepoPath: string): Promise<void> {
        try {
            const summaries = await tauriInvoke<TaskSummary[]>(
                this.zone,
                "task_load_existing",
                {
                    baseRepoPath,
                },
            );
            summaries.forEach((summary) => this.upsertTask(summary));
        } catch (error) {
            console.error("Failed to load existing worktrees", error);
        }
    }

    private async loadBranches(baseRepoPath: string): Promise<void> {
        try {
            const branches = await this.taskGit.listBranches(baseRepoPath);
            this.branchOptionsSignal.set(branches);
        } catch (error) {
            console.error("Failed to load branches", error);
            this.branchOptionsSignal.set([]);
        }
    }

    private ensureTerminalStream(
        taskId: string,
        kind: TerminalKind,
    ): Subject<string> {
        const streams = this.selectTerminalStream(kind);
        if (!streams.has(taskId)) {
            streams.set(taskId, new Subject<string>());
        }
        return streams.get(taskId)!;
    }

    private selectTerminalBuffer(kind: TerminalKind): Map<string, string> {
        return kind === "worktree"
            ? this.worktreeTerminalBuffers
            : this.terminalBuffers;
    }

    private selectTerminalStream(
        kind: TerminalKind,
    ): Map<string, Subject<string>> {
        return kind === "worktree"
            ? this.worktreeTerminalStreams
            : this.terminalStreams;
    }

    private selectLastResizeSent(
        kind: TerminalKind,
    ): Map<string, { cols: number; rows: number }> {
        return kind === "worktree"
            ? this.worktreeTerminalLastResizeSent
            : this.terminalLastResizeSent;
    }

    private ensureDiffJumpStream(taskId: string): Subject<string> {
        let stream = this.diffJumpStreams.get(taskId);
        if (!stream) {
            stream = new Subject<string>();
            this.diffJumpStreams.set(taskId, stream);
        }
        return stream;
    }

    private teardown(): void {
        while (this.unlistenFns.length > 0) {
            const unlisten = this.unlistenFns.pop();
            if (unlisten) {
                void unlisten();
            }
        }
    }

    private restoreMap<T>(
        target: Map<string, T>,
        source: Record<string, T> | undefined,
    ): void {
        target.clear();
        for (const [key, value] of Object.entries(source ?? {})) {
            target.set(key, value);
        }
    }

}

export type TaskStoreDevState = {
    baseRepo: BaseRepoInfo | null;
    tasks: TaskSummary[];
    selectedTaskId: string | null;
    branchOptions: string[];
    viewMode: "task" | "home";
    lastEditorByTaskId?: Record<string, string>;
    terminalBuffers: Record<string, string>;
    worktreeTerminalBuffers: Record<string, string>;
    worktreeTerminalOpenState: Record<string, boolean>;
    terminalSizes: Record<string, { cols: number; rows: number }>;
    worktreeTerminalSizes: Record<string, { cols: number; rows: number }>;
    lastTerminalSize: { cols: number; rows: number } | null;
    lastWorktreeTerminalSize: { cols: number; rows: number } | null;
};

export type DiffWatchState = {
    payload: DiffPayload | null;
    error: string | null;
    isLoading: boolean;
    hasLoaded: boolean;
    lastUpdated: Date | null;
};

export type DiffWatchHandle = {
    state$: Observable<DiffWatchState>;
    stop: () => Promise<void>;
};

type DiffWatcherDeps = {
    taskId: string;
    mode: DiffMode;
    refreshDelayMs: number;
    getDiff: (taskId: string, mode: DiffMode) => Promise<DiffPayload>;
    startDiffWatch: (taskId: string) => Promise<void>;
    stopDiffWatch: (taskId: string) => Promise<void>;
    listen: <T>(
        event: string,
        handler: (event: TauriEvent<T>) => void,
    ) => Promise<UnlistenFn>;
    zone: NgZone;
};

class DiffWatcher {
    private readonly stateSubject = new Subject<DiffWatchState>();
    private diffWatchUnlisten?: UnlistenFn;
    private refreshTimer: number | null = null;
    private refreshInFlight = false;
    private refreshQueued = false;
    private payload: DiffPayload | null = null;
    private error: string | null = null;
    private isLoading = false;
    private hasLoaded = false;
    private lastUpdated: Date | null = null;

    readonly state$ = this.stateSubject.asObservable();

    constructor(private readonly deps: DiffWatcherDeps) {}

    async start(): Promise<void> {
        await this.refreshDiff();
        try {
            await this.deps.startDiffWatch(this.deps.taskId);
        } catch (err) {
            console.error("Failed to start diff watcher", err);
            return;
        }
        this.diffWatchUnlisten = await this.deps.listen<{ taskId: string }>(
            "task_diff_changed",
            (event) => {
                if (event.payload.taskId !== this.deps.taskId) {
                    return;
                }
                this.scheduleDiffRefresh();
            },
        );
    }

    async stop(): Promise<void> {
        if (this.diffWatchUnlisten) {
            try {
                await this.diffWatchUnlisten();
            } catch (err) {
                console.error("Failed to unlisten diff watcher", err);
            }
            this.diffWatchUnlisten = undefined;
        }
        try {
            await this.deps.stopDiffWatch(this.deps.taskId);
        } catch (err) {
            console.error("Failed to stop diff watcher", err);
        }
        if (this.refreshTimer !== null) {
            window.clearTimeout(this.refreshTimer);
            this.refreshTimer = null;
        }
        this.refreshInFlight = false;
        this.refreshQueued = false;
    }

    private scheduleDiffRefresh(): void {
        if (this.refreshTimer !== null) {
            window.clearTimeout(this.refreshTimer);
        }
        this.refreshTimer = window.setTimeout(() => {
            this.refreshTimer = null;
            void this.refreshDiff();
        }, this.deps.refreshDelayMs);
    }

    private async refreshDiff(): Promise<void> {
        if (this.refreshInFlight) {
            this.refreshQueued = true;
            return;
        }
        this.refreshInFlight = true;
        this.refreshQueued = false;
        if (!this.hasLoaded) {
            this.isLoading = true;
            this.emitState();
        }
        try {
            const payload = await this.deps.getDiff(
                this.deps.taskId,
                this.deps.mode,
            );
            this.payload = payload;
            this.error = null;
            this.lastUpdated = new Date();
            this.emitState();
        } catch (err) {
            const message =
                err instanceof Error
                    ? err.message
                    : "Unable to load diff. The git repository may be inaccessible.";
            this.error = message;
            this.emitState();
        } finally {
            this.hasLoaded = true;
            this.isLoading = false;
            this.emitState();
            this.refreshInFlight = false;
            if (this.refreshQueued) {
                this.scheduleDiffRefresh();
            }
        }
    }

    private emitState(): void {
        const snapshot: DiffWatchState = {
            payload: this.payload,
            error: this.error,
            isLoading: this.isLoading,
            hasLoaded: this.hasLoaded,
            lastUpdated: this.lastUpdated,
        };
        this.deps.zone.run(() => {
            this.stateSubject.next(snapshot);
        });
    }
}
