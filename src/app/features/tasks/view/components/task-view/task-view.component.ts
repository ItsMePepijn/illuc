import { CommonModule } from "@angular/common";
import {
    ChangeDetectorRef,
    Component,
    EventEmitter,
    Input,
    Output,
    ViewChild,
    ElementRef,
    NgZone,
} from "@angular/core";
import { FormsModule } from "@angular/forms";
import { AgentKind, TaskSummary, BaseRepoInfo } from "../../../models";
import { parseTitleParts, TitleParts } from "../../../title.utils";
import { TaskTerminalComponent } from "../../../terminal/components/task-terminal/task-terminal.component";
import { TaskDiffComponent } from "../../../git/components/task-diff/task-diff.component";
import { TaskActionButtonComponent } from "../../../actions/components/task-action-button/task-action-button.component";
import { IconGitCommitComponent } from "../../../actions/components/icon-git-commit/icon-git-commit.component";
import { IconGitPushComponent } from "../../../actions/components/icon-git-push/icon-git-push.component";
import { IconTrashBinComponent } from "../../../actions/components/icon-trash-bin/icon-trash-bin.component";
import { IconStopSquareComponent } from "../../../actions/components/icon-stop-square/icon-stop-square.component";
import { OpenVsCodeButtonComponent } from "../../../workspace/components/open-vscode-button/open-vscode-button.component";
import { OpenTerminalButtonComponent } from "../../../workspace/components/open-terminal-button/open-terminal-button.component";
import { StartAgentDropdownComponent } from "../../../agents/components/start-agent-dropdown/start-agent-dropdown.component";
import { LoadingButtonComponent } from "../../../../../shared/components/loading-button/loading-button.component";
import { TaskHomeDashboardComponent } from "../../../home/components/task-home-dashboard/task-home-dashboard.component";
import { TaskGettingStartedComponent } from "../../../home/components/task-getting-started/task-getting-started.component";
import { TaskStore } from "../../../task.store";

@Component({
    selector: "app-task-view",
    standalone: true,
    imports: [
        CommonModule,
        FormsModule,
        TaskTerminalComponent,
        TaskDiffComponent,
        TaskActionButtonComponent,
        IconGitCommitComponent,
        IconGitPushComponent,
        IconTrashBinComponent,
        IconStopSquareComponent,
        OpenVsCodeButtonComponent,
        OpenTerminalButtonComponent,
        StartAgentDropdownComponent,
        LoadingButtonComponent,
        TaskHomeDashboardComponent,
        TaskGettingStartedComponent,
    ],
    templateUrl: "./task-view.component.html",
    styleUrl: "./task-view.component.css",
})
export class TaskViewComponent {
    @Input() task: TaskSummary | null = null;
    @Input() baseRepo: BaseRepoInfo | null = null;
    @Input() showGettingStarted = false;
    @Input() backgroundMode = false;
    @Input() startLoading = false;
    @Input() stopLoading = false;
    @Input() discardLoading = false;
    @Input() selectRepoLoading = false;
    @Input() selectRepoError = "";
    activePane: "terminal" | "diff" = "terminal";
    isShellTerminalOpen = false;
    isShellResizing = false;
    shellTerminalHeight = 260;
    private readonly minShellHeight = 160;
    @ViewChild("shellTerminal") shellTerminal?: TaskTerminalComponent;
    @ViewChild("shellDock") shellDock?: ElementRef<HTMLDivElement>;
    @ViewChild("taskDetail") taskDetail?: ElementRef<HTMLElement>;
    @Output() startTask = new EventEmitter<{
        taskId: string;
        agent: AgentKind;
    }>();
    @Output() stopTask = new EventEmitter<string>();
    @Output() discardTask = new EventEmitter<string>();
    @Output() selectBaseRepo = new EventEmitter<void>();
    showCommitModal = false;
    showPushModal = false;
    commitMessage = "";
    commitStageAll = true;
    commitError = "";
    pushRemote = "origin";
    pushBranch = "";
    pushSetUpstream = true;
    pushError = "";
    isCommitting = false;
    isPushing = false;
    readonly agentKind = AgentKind;

    constructor(
        private readonly taskStore: TaskStore,
        private readonly zone: NgZone,
        private readonly cdr: ChangeDetectorRef,
    ) {}

    ngOnChanges(): void {
        if (this.task?.taskId) {
            this.isShellTerminalOpen = this.taskStore.isWorktreeTerminalOpen(
                this.task.taskId,
            );
        } else {
            this.isShellTerminalOpen = false;
        }
        if (!this.isRunning()) {
            this.activePane = "terminal";
        }
    }

    statusLabel(): string {
        return this.task?.status.replace(/_/g, " ") ?? "";
    }

    canStart(): boolean {
        return (
            !!this.task &&
            ["STOPPED", "COMPLETED", "FAILED"].includes(this.task.status)
        );
    }

    isRunning(): boolean {
        return (
            !!this.task &&
            ["IDLE", "AWAITING_APPROVAL", "WORKING"].includes(this.task.status)
        );
    }

    titleParts(): TitleParts | null {
        if (!this.task) {
            return null;
        }
        return parseTitleParts(this.task.title);
    }

    startWith(agent: AgentKind): void {
        if (!this.task) {
            return;
        }
        this.taskStore.clearTerminalBuffer(this.task.taskId, "agent");
        this.startTask.emit({ taskId: this.task.taskId, agent });
    }

    onStop(): void {
        if (this.task) {
            this.stopTask.emit(this.task.taskId);
        }
    }

    onDiscard(): void {
        if (this.task) {
            this.discardTask.emit(this.task.taskId);
        }
    }

    openCommitModal(): void {
        if (!this.task) {
            return;
        }
        this.commitMessage = "";
        this.commitStageAll = true;
        this.commitError = "";
        this.showCommitModal = true;
    }

    closeCommitModal(): void {
        this.showCommitModal = false;
        this.commitMessage = "";
        this.commitError = "";
    }

    async submitCommit(): Promise<void> {
        const task = this.task;
        if (!task) {
            return;
        }
        if (this.isCommitting) {
            return;
        }
        if (!this.commitMessage.trim()) {
            this.commitError = "Commit message is required.";
            return;
        }
        this.commitError = "";
        this.isCommitting = true;
        try {
            await this.taskStore.commitTask(
                task.taskId,
                this.commitMessage.trim(),
                this.commitStageAll,
            );
            this.closeCommitModal();
        } catch (error: unknown) {
            this.commitError = this.describeError(
                error,
                "Unable to commit changes.",
            );
        } finally {
            this.isCommitting = false;
            this.cdr.detectChanges();
        }
    }

    openPushModal(): void {
        if (!this.task) {
            return;
        }
        this.pushRemote = "origin";
        this.pushBranch = this.task.branchName;
        this.pushSetUpstream = true;
        this.pushError = "";
        this.showPushModal = true;
    }

    closePushModal(): void {
        this.showPushModal = false;
        this.pushError = "";
    }

    async submitPush(): Promise<void> {
        const task = this.task;
        if (!task) {
            return;
        }
        if (this.isPushing) {
            return;
        }
        this.pushError = "";
        this.isPushing = true;
        try {
            await this.taskStore.pushTask(
                task.taskId,
                this.pushRemote.trim() || "origin",
                this.pushBranch.trim() || task.branchName,
                this.pushSetUpstream,
            );
            this.closePushModal();
        } catch (error: unknown) {
            this.pushError = this.describeError(
                error,
                "Unable to push changes.",
            );
        } finally {
            this.isPushing = false;
            this.cdr.detectChanges();
        }
    }

    onSelectBaseRepo(): void {
        this.selectBaseRepo.emit();
    }

    setActivePane(pane: "terminal" | "diff"): void {
        this.activePane = pane;
    }

    toggleShellTerminal(): void {
        this.isShellTerminalOpen = !this.isShellTerminalOpen;
        if (this.task?.taskId) {
            this.taskStore.setWorktreeTerminalOpen(
                this.task.taskId,
                this.isShellTerminalOpen,
            );
        }
    }

    onShellHeaderMouseDown(event: MouseEvent): void {
        if (!this.isShellTerminalOpen) {
            return;
        }
        this.startShellResize(event);
    }

    onShellHeaderClick(): void {
        if (!this.isShellTerminalOpen) {
            this.toggleShellTerminal();
        }
    }

    startShellResize(event: MouseEvent): void {
        if (!this.isShellTerminalOpen) {
            return;
        }
        event.preventDefault();
        this.isShellResizing = true;
        const startY = event.clientY;
        const startHeight = this.shellTerminalHeight;
        let latestHeight = startHeight;
        let rafId: number | null = null;
        const dockEl = this.shellDock?.nativeElement;
        const containerHeight =
            this.taskDetail?.nativeElement.clientHeight ?? window.innerHeight;
        const maxShellHeight = Math.max(
            this.minShellHeight,
            containerHeight - 16,
        );

        const handleMove = (moveEvent: MouseEvent) => {
            const delta = startY - moveEvent.clientY;
            const next = Math.max(
                this.minShellHeight,
                Math.min(maxShellHeight, startHeight + delta),
            );
            latestHeight = next;
            if (rafId === null) {
                rafId = requestAnimationFrame(() => {
                    if (dockEl) {
                        dockEl.style.height = `${latestHeight}px`;
                    } else {
                        this.shellTerminalHeight = latestHeight;
                    }
                    rafId = null;
                });
            }
        };

        const handleUp = () => {
            window.removeEventListener("mousemove", handleMove);
            window.removeEventListener("mouseup", handleUp);
            if (rafId !== null) {
                cancelAnimationFrame(rafId);
                rafId = null;
            }
            this.zone.run(() => {
                this.shellTerminalHeight = latestHeight;
                this.isShellResizing = false;
                this.shellTerminal?.forceBackendResizeNow(true);
            });
        };

        this.zone.runOutsideAngular(() => {
            window.addEventListener("mousemove", handleMove);
            window.addEventListener("mouseup", handleUp);
        });
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

}
