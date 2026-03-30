import { CommonModule } from "@angular/common";
import {
    ChangeDetectorRef,
    Component,
    EventEmitter,
    HostListener,
    Input,
    Output,
    ViewChild,
    ElementRef,
    NgZone,
} from "@angular/core";
import { FormsModule } from "@angular/forms";
import { AgentKind, TaskSummary, BaseRepoInfo } from "../../../models";
import { parseTitleParts, TitleParts } from "../../../title.utils";
import { AgentTuiComponent } from "../../../agent-tui/components/tui/tui.component";
import { TerminalSessionComponent } from "../../../terminal/components/terminal-session/terminal-session.component";
import { TaskDiffComponent } from "../../../git/components/task-diff/task-diff.component";
import { TaskActionButtonComponent } from "../../../actions/components/task-action-button/task-action-button.component";
import { IconGitCommitComponent } from "../../../actions/components/icon-git-commit/icon-git-commit.component";
import { IconGitMergeComponent } from "../../../actions/components/icon-git-merge/icon-git-merge.component";
import { IconGitPushComponent } from "../../../actions/components/icon-git-push/icon-git-push.component";
import { IconTrashBinComponent } from "../../../actions/components/icon-trash-bin/icon-trash-bin.component";
import { IconStopSquareComponent } from "../../../actions/components/icon-stop-square/icon-stop-square.component";
import { LaunchEditorDropdownComponent } from "../../../workspace/components/launch-editor-dropdown/launch-editor-dropdown.component";
import { OpenTerminalButtonComponent } from "../../../workspace/components/open-terminal-button/open-terminal-button.component";
import { StartAgentDropdownComponent } from "../../../launcher/components/start-agent-dropdown/start-agent-dropdown.component";
import { AgentChatComponent } from "../../../agent-chat/components/chat/chat.component";
import { IconLoadingButtonComponent } from "../../../../../shared/components/icon-loading-button/icon-loading-button.component";
import { LoadingButtonComponent } from "../../../../../shared/components/loading-button/loading-button.component";
import { TaskHomeDashboardComponent } from "../../../home/components/task-home-dashboard/task-home-dashboard.component";
import { TaskGettingStartedComponent } from "../../../home/components/task-getting-started/task-getting-started.component";
import { TaskStore } from "../../../task.store";
import { AgentChatStore } from "../../../agent-chat/agent-chat.store";
import { TokenUsageDashboardComponent } from "../../../../token-usage/components/token-usage-dashboard/token-usage-dashboard.component";

@Component({
    selector: "app-task-view",
    standalone: true,
    imports: [
        CommonModule,
        FormsModule,
        AgentTuiComponent,
        TerminalSessionComponent,
        TaskDiffComponent,
        TaskActionButtonComponent,
        IconGitCommitComponent,
        IconGitMergeComponent,
        IconGitPushComponent,
        IconTrashBinComponent,
        IconStopSquareComponent,
        LaunchEditorDropdownComponent,
        OpenTerminalButtonComponent,
        StartAgentDropdownComponent,
        AgentChatComponent,
        IconLoadingButtonComponent,
        LoadingButtonComponent,
        TaskHomeDashboardComponent,
        TaskGettingStartedComponent,
        TokenUsageDashboardComponent,
    ],
    templateUrl: "./task-view.component.html",
    styleUrl: "./task-view.component.css",
})
export class TaskViewComponent {
    @Input() task: TaskSummary | null = null;
    @Input() baseRepo: BaseRepoInfo | null = null;
    @Input() showGettingStarted = false;
    @Input() homePage: "timeTracking" | "tokenUsage" = "timeTracking";
    @Input() backgroundMode = false;
    @Input() startLoading = false;
    @Input() stopLoading = false;
    @Input() discardLoading = false;
    @Input() selectRepoLoading = false;
    @Input() selectRepoError = "";
    activePane: "terminal" | "diff" = "terminal";
    agentActionsMenuOpen = false;
    startingNewChat = false;
    compacting = false;
    rollingBack = false;
    isShellTerminalOpen = false;
    isShellResizing = false;
    shellTerminalHeight = 260;
    private readonly minShellHeight = 160;
    @ViewChild("shellTerminal") shellTerminal?: TerminalSessionComponent;
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
    showMergeModal = false;
    pendingAgentChatAction: "new-chat" | "rollback" | null = null;
    pendingPostCommitAction: "merge" | null = null;
    commitMessage = "";
    commitStageAll = true;
    commitError = "";
    pushRemote = "origin";
    pushBranch = "";
    pushSetUpstream = true;
    pushError = "";
    isCommitting = false;
    isPreparingMerge = false;
    isMerging = false;
    isPushing = false;
    mergeError = "";
    mergePushAfter = true;
    mergeRequiresAcknowledgement = false;
    readonly agentKind = AgentKind;

    constructor(
        private readonly taskStore: TaskStore,
        private readonly agentChatStore: AgentChatStore,
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
        this.loadGuiAgentCapabilities();
        if (!this.task || !this.hasAgentChatActionsMenu(this.task) || !this.isRunning()) {
            this.agentActionsMenuOpen = false;
        }
    }

    @HostListener("document:click")
    closeAgentActionsMenu(): void {
        this.agentActionsMenuOpen = false;
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

    async compactAgentChat(): Promise<void> {
        if (
            !this.task ||
            !this.supportsThreadHistoryActions(this.task) ||
            this.compacting
        ) {
            return;
        }
        this.agentActionsMenuOpen = false;
        this.compacting = true;
        try {
            await this.agentChatStore.compactThread(this.task.taskId);
        } finally {
            this.compacting = false;
            this.cdr.markForCheck();
        }
    }

    async startNewAgentChat(): Promise<void> {
        if (!this.task || !this.supportsNewChatAction(this.task) || this.startingNewChat) {
            return;
        }
        this.agentActionsMenuOpen = false;
        this.pendingAgentChatAction = "new-chat";
    }

    async confirmStartNewAgentChat(): Promise<void> {
        if (!this.task || !this.supportsNewChatAction(this.task) || this.startingNewChat) {
            return;
        }
        this.pendingAgentChatAction = null;
        this.startingNewChat = true;
        try {
            await this.agentChatStore.newChat(this.task.taskId);
        } finally {
            this.startingNewChat = false;
            this.cdr.markForCheck();
        }
    }

    async rollbackAgentChatTurn(): Promise<void> {
        if (
            !this.task ||
            !this.supportsThreadHistoryActions(this.task) ||
            this.rollingBack
        ) {
            return;
        }
        this.agentActionsMenuOpen = false;
        this.pendingAgentChatAction = "rollback";
    }

    async confirmRollbackAgentChatTurn(): Promise<void> {
        if (
            !this.task ||
            !this.supportsThreadHistoryActions(this.task) ||
            this.rollingBack
        ) {
            return;
        }
        this.pendingAgentChatAction = null;
        this.rollingBack = true;
        try {
            await this.agentChatStore.rollbackThread(this.task.taskId, 1);
        } finally {
            this.rollingBack = false;
            this.cdr.markForCheck();
        }
    }

    closeAgentChatActionConfirmModal(): void {
        if (this.startingNewChat || this.rollingBack) {
            return;
        }
        this.pendingAgentChatAction = null;
    }

    agentChatActionConfirmTitle(): string {
        return this.pendingAgentChatAction === "new-chat"
            ? "Start new chat?"
            : "Rollback last turn?";
    }

    agentChatActionConfirmMessage(): string {
        return this.pendingAgentChatAction === "new-chat"
            ? "This clears the current agent chat history for this task, but does not revert file changes."
            : "Rollback removes the latest turn from the agent chat history, but it does not revert file changes.";
    }

    agentChatActionConfirmButtonLabel(): string {
        if (this.pendingAgentChatAction === "new-chat") {
            return this.startingNewChat ? "Starting..." : "Start new chat";
        }
        return this.rollingBack ? "Rolling back..." : "Rollback";
    }

    async confirmAgentChatAction(): Promise<void> {
        if (this.pendingAgentChatAction === "new-chat") {
            await this.confirmStartNewAgentChat();
            return;
        }
        if (this.pendingAgentChatAction === "rollback") {
            await this.confirmRollbackAgentChatTurn();
        }
    }

    onDiscard(): void {
        if (this.task) {
            this.discardTask.emit(this.task.taskId);
        }
    }

    toggleAgentActionsMenu(event: MouseEvent): void {
        event.preventDefault();
        event.stopPropagation();
        this.agentActionsMenuOpen = !this.agentActionsMenuOpen;
    }

    openCommitModal(nextAction: "merge" | null = null): void {
        if (!this.task) {
            return;
        }
        this.pendingPostCommitAction = nextAction;
        this.commitMessage = "";
        this.commitStageAll = true;
        this.commitError = "";
        this.showCommitModal = true;
    }

    closeCommitModal(): void {
        this.showCommitModal = false;
        this.commitMessage = "";
        this.commitError = "";
        this.pendingPostCommitAction = null;
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
            const nextAction = this.pendingPostCommitAction;
            this.showCommitModal = false;
            this.commitMessage = "";
            this.commitError = "";
            this.pendingPostCommitAction = null;
            if (nextAction === "merge") {
                this.openMergeModal();
            }
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

    mergeTargetBranch(): string {
        return this.baseRepo?.currentBranch?.trim() ?? "";
    }

    canMerge(task: TaskSummary | null = this.task): boolean {
        const targetBranch = this.mergeTargetBranch();
        return !!task && !!targetBranch && task.branchName !== targetBranch;
    }

    async openMergeFlow(): Promise<void> {
        const task = this.task;
        if (!task || !this.canMerge(task) || this.isPreparingMerge) {
            return;
        }
        this.isPreparingMerge = true;
        this.mergeError = "";
        try {
            const hasChanges = await this.taskStore.hasUncommittedChanges(task.taskId);
            if (hasChanges) {
                this.openCommitModal("merge");
                return;
            }
            this.openMergeModal();
        } catch (error: unknown) {
            this.mergeError = this.describeError(error, "Unable to prepare merge.");
            this.showMergeModal = true;
        } finally {
            this.isPreparingMerge = false;
            this.cdr.detectChanges();
        }
    }

    openMergeModal(): void {
        if (!this.canMerge()) {
            return;
        }
        this.mergeError = "";
        this.mergePushAfter = true;
        this.mergeRequiresAcknowledgement = false;
        this.showMergeModal = true;
    }

    closeMergeModal(): void {
        if (this.isMerging) {
            return;
        }
        this.resetMergeModalState();
    }

    private resetMergeModalState(): void {
        this.showMergeModal = false;
        this.mergeError = "";
        this.mergePushAfter = true;
        this.mergeRequiresAcknowledgement = false;
    }

    async submitMerge(): Promise<void> {
        const task = this.task;
        const targetBranch = this.mergeTargetBranch();
        if (!task || !this.canMerge(task) || this.isMerging) {
            return;
        }
        this.mergeError = "";
        this.mergeRequiresAcknowledgement = false;
        this.isMerging = true;
        try {
            await this.taskStore.mergeTask(
                task.taskId,
                targetBranch,
                this.mergePushAfter,
            );
            this.resetMergeModalState();
        } catch (error: unknown) {
            const message = this.describeError(
                error,
                `Unable to merge ${task.branchName} into ${targetBranch}.`,
            );
            this.mergeError = message;
            this.mergeRequiresAcknowledgement =
                message.startsWith("Merged into ") &&
                message.includes("locally, but pushing");
        } finally {
            this.isMerging = false;
            this.cdr.detectChanges();
        }
    }

    commitModalTitle(): string {
        return this.pendingPostCommitAction === "merge"
            ? "Commit changes before merging"
            : "Commit changes";
    }

    commitModalSubtitle(): string {
        if (this.pendingPostCommitAction === "merge" && this.task) {
            return `Commit your task changes before merging ${this.task.branchName} into ${this.mergeTargetBranch()}.`;
        }
        return "Write a commit message for this task.";
    }

    commitModalSubmitLabel(): string {
        if (this.pendingPostCommitAction === "merge") {
            return this.isCommitting ? "Committing..." : "Commit and continue";
        }
        return this.isCommitting ? "Committing..." : "Commit";
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

    isAgentChatTask(task: TaskSummary): boolean {
        return task.usesAgentChat;
    }

    supportsNewChatAction(task: TaskSummary): boolean {
        return this.agentChatStore.getCapabilities(task.taskId).supportsNewChat;
    }

    supportsThreadHistoryActions(task: TaskSummary): boolean {
        return this.agentChatStore.getCapabilities(task.taskId)
            .supportsThreadHistory;
    }

    hasAgentChatActionsMenu(task: TaskSummary): boolean {
        return (
            this.supportsNewChatAction(task) ||
            this.supportsThreadHistoryActions(task)
        );
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

    private loadGuiAgentCapabilities(): void {
        const task = this.task;
        if (!task || !this.isRunning() || !this.isAgentChatTask(task)) {
            return;
        }
        const taskId = task.taskId;
        void this.agentChatStore
            .ensureCapabilities(taskId)
            .then(() => {
                if (this.task?.taskId !== taskId) {
                    return;
                }
                if (!this.hasAgentChatActionsMenu(this.task)) {
                    this.agentActionsMenuOpen = false;
                }
                this.cdr.markForCheck();
            })
            .catch(() => {
                // Keep fallback UI state when capability metadata is unavailable.
            });
    }

}
