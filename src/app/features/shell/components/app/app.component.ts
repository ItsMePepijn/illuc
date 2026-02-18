import { CommonModule } from "@angular/common";
import { Component, NgZone, effect, signal } from "@angular/core";
import { FormsModule } from "@angular/forms";
import { NavigationEnd, Router } from "@angular/router";
import { open } from "@tauri-apps/plugin-dialog";
import { filter } from "rxjs/operators";
import { TaskSidebarComponent } from "../../../tasks/sidebar/components/task-sidebar/task-sidebar.component";
import { TaskViewComponent } from "../../../tasks/view/components/task-view/task-view.component";
import { AgentKind, TaskSummary } from "../../../tasks/task.models";
import { deriveTitleFromBranch } from "../../../tasks/title.utils";
import { TaskStore } from "../../../tasks/task.store";
import { LauncherService } from "../../../launcher/launcher.service";
import { LoadingButtonComponent } from "../../../../shared/components/loading-button/loading-button.component";
import { TaskTimeTrackingService } from "../../../time-tracking/task-time-tracking.service";
import { ThemeService } from "../../theme.service";
import { wrapPromiseInZone } from "../../../../shared/tauri/tauri-zone";

type ConfirmDiscardState = {
    taskId: string;
    title: string;
    branch: string;
    error: string;
    hasChanges: boolean;
};

@Component({
    selector: "app-shell",
    standalone: true,
    imports: [
        CommonModule,
        FormsModule,
        TaskSidebarComponent,
        TaskViewComponent,
        LoadingButtonComponent,
    ],
    templateUrl: "./app.component.html",
    styleUrl: "./app.component.css",
})
export class AppComponent {
    showCreateModal = false;
    branchNameInput = "";
    branchNameError = "";
    readonly confirmDiscard = signal<ConfirmDiscardState | null>(null);
    baseBranchSelection = "";
    isSelectingRepo = false;
    readonly repoSelectionError = signal("");
    isCreatingTask = false;
    private mountedTaskIds: string[] = [];
    private readonly startingTaskIds = new Set<string>();
    private readonly stoppingTaskIds = new Set<string>();
    private readonly discardingTaskIds = new Set<string>();

    constructor(
        public readonly taskStore: TaskStore,
        private readonly launcher: LauncherService,
        private readonly timeTracking: TaskTimeTrackingService,
        private readonly themeService: ThemeService,
        private readonly zone: NgZone,
        private readonly router: Router,
    ) {
        void this.themeService.applyFromSettings();
        effect(() => {
            const baseRepoPath = this.taskStore.baseRepo()?.path ?? null;
            const task = this.taskStore.selectedTask();
            void this.timeTracking.syncContext(
                baseRepoPath,
                task
                    ? {
                          taskId: task.taskId,
                          branchName: task.branchName,
                          title: task.title,
                          baseRepoPath: task.baseRepoPath,
                      }
                    : null,
            );
        });
        effect(() => {
            const tasks = this.taskStore.tasks();
            const selectedTaskId = this.taskStore.selectedTaskId();
            const nextMounted = new Set<string>();
            if (selectedTaskId) {
                nextMounted.add(selectedTaskId);
            }
            for (const task of tasks) {
                if (this.isTaskRunning(task)) {
                    nextMounted.add(task.taskId);
                }
            }
            this.mountedTaskIds = this.mountedTaskIds.filter((taskId) =>
                nextMounted.has(taskId),
            );
            for (const taskId of nextMounted) {
                if (!this.mountedTaskIds.includes(taskId)) {
                    this.mountedTaskIds.push(taskId);
                }
            }
        });
        this.syncSelectionFromRoute(this.router.url);
        this.router.events
            .pipe(filter((event) => event instanceof NavigationEnd))
            .subscribe((event) => {
                this.syncSelectionFromRoute(
                    (event as NavigationEnd).urlAfterRedirects,
                );
            });
    }

    tasks() {
        return this.taskStore.tasks();
    }

    selectedTask() {
        return this.taskStore.selectedTask();
    }

    selectedTaskId() {
        return this.taskStore.selectedTaskId();
    }

    baseRepo() {
        return this.taskStore.baseRepo();
    }

    branchOptions() {
        return this.taskStore.branches();
    }

    async browseForRepo(): Promise<void> {
        if (this.isSelectingRepo) {
            return;
        }
        this.zone.run(() => {
            this.repoSelectionError.set("");
            this.isSelectingRepo = true;
        });
        try {
            const selection = await wrapPromiseInZone(this.zone, () => open({
                directory: true,
                multiple: false,
                title: "Select base repository",
            }));
            if (typeof selection === "string") {
                await this.loadBaseRepo(selection);
            }
        } finally {
            this.isSelectingRepo = false;
        }
    }

    private async loadBaseRepo(path: string): Promise<void> {
        try {
            await this.taskStore.selectBaseRepo(path);
            this.zone.run(() => {
                this.repoSelectionError.set("");
            });
            if (this.taskStore.tasks().length === 0) {
                this.taskStore.selectHome();
                await this.navigateByUrl("/getting-started", true);
                return;
            }
            this.taskStore.selectHome();
            if (this.router.url !== "/dashboard") {
                await this.navigateByUrl("/dashboard", true);
            }
        } catch (error: unknown) {
            const message = this.describeBaseRepoError(error);
            this.zone.run(() => {
                this.repoSelectionError.set(message);
            });
            console.error(message);
        }
    }

    openCreateTaskModal(): void {
        if (!this.baseRepo()) {
            console.error("Select a base repository before creating tasks.");
            return;
        }
        this.branchNameInput = "";
        this.branchNameError = "";
        this.baseBranchSelection = this.taskStore.defaultBaseBranch() ?? "";
        this.showCreateModal = true;
    }

    closeCreateTaskModal(): void {
        this.showCreateModal = false;
        this.branchNameInput = "";
        this.branchNameError = "";
        this.baseBranchSelection = "";
    }

    async submitNewTask(): Promise<void> {
        const branch = this.branchNameInput.trim();
        if (!branch) {
            this.branchNameError = "Branch name is required.";
            return;
        }
        if (!this.baseRepo()) {
            this.branchNameError = "Select a base repository first.";
            return;
        }
        if (this.isCreatingTask) {
            return;
        }
        this.branchNameError = "";
        const title = deriveTitleFromBranch(branch);
        this.isCreatingTask = true;
        try {
            const created = await this.taskStore.createTask(
                branch,
                title,
                this.baseBranchSelection,
            );
            await this.navigateByUrl(`/tasks/${created.taskId}`);
            this.closeCreateTaskModal();
        } catch (error: unknown) {
            this.branchNameError = this.describeError(
                error,
                "Unable to create task.",
            );
        } finally {
            this.isCreatingTask = false;
        }
    }

    async startTask(payload: {
        taskId: string;
        agent: AgentKind;
    }): Promise<void> {
        if (this.startingTaskIds.has(payload.taskId)) {
            return;
        }
        this.startingTaskIds.add(payload.taskId);
        try {
            await this.taskStore.startTask(payload.taskId, payload.agent);
        } catch (error: unknown) {
            console.error(this.describeError(error, "Unable to start task."));
        } finally {
            this.startingTaskIds.delete(payload.taskId);
        }
    }

    async stopTask(taskId: string): Promise<void> {
        if (this.stoppingTaskIds.has(taskId)) {
            return;
        }
        this.stoppingTaskIds.add(taskId);
        try {
            await this.taskStore.stopTask(taskId);
        } catch (error: unknown) {
            console.error(this.describeError(error, "Unable to stop task."));
        } finally {
            this.stoppingTaskIds.delete(taskId);
        }
    }

    async discardTask(taskId: string): Promise<void> {
        if (
            this.discardingTaskIds.has(taskId)
        ) {
            return;
        }
        this.discardingTaskIds.add(taskId);
        try {
            const hasChanges =
                await this.taskStore.hasUncommittedChanges(taskId);
            if (!hasChanges) {
                try {
                    await this.taskStore.discardTask(taskId);
                    await this.navigateAfterTaskRemoval();
                } catch (error: unknown) {
                    console.error(
                        this.describeError(error, "Unable to discard task."),
                    );
                }
                return;
            }
            const task =
                this.taskStore.tasks().find((wf) => wf.taskId === taskId) ??
                null;
            this.confirmDiscard.set({
                taskId,
                title: task?.title ?? "Selected task",
                branch: task?.branchName ?? "",
                error: "",
                hasChanges: true,
            });
        } catch (error: unknown) {
            const task =
                this.taskStore.tasks().find((wf) => wf.taskId === taskId) ??
                null;
            this.confirmDiscard.set({
                taskId,
                title: task?.title ?? "Selected task",
                branch: task?.branchName ?? "",
                error: this.describeError(
                    error,
                    "Unable to check for uncommitted changes.",
                ),
                hasChanges: true,
            });
        } finally {
            this.discardingTaskIds.delete(taskId);
        }
    }

    selectTask(taskId: string): void {
        const target = `/tasks/${taskId}`;
        if (this.router.url !== target) {
            void this.navigateByUrl(target);
        }
    }

    selectHome(): void {
        if (this.router.url !== "/dashboard") {
            void this.navigateByUrl("/dashboard");
        }
    }

    isGettingStartedRoute(): boolean {
        return this.currentRouteFirstSegment() === "getting-started";
    }

    isDashboardRoute(): boolean {
        return this.currentRouteFirstSegment() === "dashboard";
    }

    cancelDiscardTask(): void {
        this.confirmDiscard.set(null);
    }

    async confirmDiscardTask(): Promise<void> {
        const snapshot = this.confirmDiscard();
        if (!snapshot) {
            return;
        }
        // Close immediately; run the discard in the background while the task row
        // shows a loading spinner.
        this.confirmDiscard.set(null);
        this.startDiscardInBackground(snapshot);
    }

    private startDiscardInBackground(snapshot: ConfirmDiscardState): void {
        const taskId = snapshot.taskId;
        if (this.discardingTaskIds.has(taskId)) {
            return;
        }
        this.discardingTaskIds.add(taskId);
        void (async () => {
            try {
                await this.taskStore.discardTask(taskId);
                await this.navigateAfterTaskRemoval();
            } catch (error: unknown) {
                const message = this.describeError(
                    error,
                    "Unable to discard task.",
                );
                console.error(message);
                // Re-open the modal with the error so the user can retry / cancel.
                this.zone.run(() => {
                    this.confirmDiscard.set({
                        ...snapshot,
                        error: message,
                    });
                });
            } finally {
                this.zone.run(() => {
                    this.discardingTaskIds.delete(taskId);
                });
            }
        })();
    }

    async openInExplorer(event: Event, path: string): Promise<void> {
        event.preventDefault();
        try {
            await this.launcher.openInExplorer(path);
        } catch (error) {
            console.error("Failed to open explorer", error);
        }
    }

    private async navigateToGettingStartedIfNoTasks(): Promise<void> {
        if (!this.baseRepo() || this.taskStore.tasks().length > 0) {
            return;
        }
        if (this.router.url === "/getting-started") {
            return;
        }
        await this.navigateByUrl("/getting-started", true);
    }

    private syncSelectionFromRoute(url: string): void {
        const tree = this.router.parseUrl(url);
        const primary = tree.root.children["primary"];
        const segments = primary?.segments ?? [];
        const first = segments[0]?.path ?? null;
        const second = segments[1]?.path ?? null;
        const routeTaskId = first === "tasks" && second ? second : null;
        if (routeTaskId) {
            this.taskStore.selectTask(routeTaskId);
            return;
        }
        this.taskStore.selectHome();
    }

    private currentRouteFirstSegment(): string | null {
        const tree = this.router.parseUrl(this.router.url);
        const primary = tree.root.children["primary"];
        return primary?.segments[0]?.path ?? null;
    }

    private async navigateAfterTaskRemoval(): Promise<void> {
        const nextSelectedTaskId = this.taskStore.selectedTaskId();
        if (nextSelectedTaskId) {
            const target = `/tasks/${nextSelectedTaskId}`;
            if (this.router.url !== target) {
                await this.navigateByUrl(target, true);
            }
            return;
        }
        await this.navigateToGettingStartedIfNoTasks();
    }

    private navigateByUrl(url: string, replaceUrl = false): Promise<boolean> {
        return this.zone.run(() =>
            this.router.navigateByUrl(
                url,
                replaceUrl ? { replaceUrl: true } : undefined,
            ),
        );
    }

    private describeError(error: unknown, fallback: string): string {
        if (typeof error === "string") {
            return error;
        }
        if (error && typeof error === "object" && "message" in error) {
            return String((error as { message: string }).message);
        }
        return fallback;
    }

    private describeBaseRepoError(error: unknown): string {
        const message = this.describeError(
            error,
            "Unable to open the selected repository.",
        );
        if (
            message
                .toLowerCase()
                .includes("not a git repository")
        ) {
            return "That folder is not a Git repository. Choose another directory.";
        }
        return message;
    }

    isStartingTask(taskId: string | null | undefined): boolean {
        return !!taskId && this.startingTaskIds.has(taskId);
    }

    isStoppingTask(taskId: string | null | undefined): boolean {
        return !!taskId && this.stoppingTaskIds.has(taskId);
    }

    stoppingTasks(): Set<string> {
        return this.stoppingTaskIds;
    }

    discardingTasks(): Set<string> {
        return this.discardingTaskIds;
    }

    isDiscardingTaskFor(taskId: string | null | undefined): boolean {
        return !!taskId && this.discardingTaskIds.has(taskId);
    }

    mountedTasks(): { taskId: string; task: TaskSummary }[] {
        const taskMap = new Map(
            this.taskStore.tasks().map((task) => [task.taskId, task]),
        );
        return this.mountedTaskIds
            .map((taskId) => ({ taskId, task: taskMap.get(taskId) ?? null }))
            .filter(
                (
                    entry,
                ): entry is {
                    taskId: string;
                    task: TaskSummary;
                } => !!entry.task,
            );
    }

    trackMountedTask(
        _: number,
        entry: { taskId: string; task: TaskSummary },
    ): string {
        return entry.taskId;
    }

    private isTaskRunning(task: TaskSummary): boolean {
        return ["IDLE", "AWAITING_APPROVAL", "WORKING"].includes(task.status);
    }
}
