import { CommonModule } from "@angular/common";
import { Component, EventEmitter, Input, Output } from "@angular/core";
import { FormsModule } from "@angular/forms";
import { AgentKind, TaskSummary, BaseRepoInfo } from "../../task.models";
import { parseTitleParts, TitleParts } from "../../title.utils";
import { TaskTerminalComponent } from "../task-terminal/task-terminal.component";
import { TaskDiffComponent } from "../task-diff/task-diff.component";
import { TaskActionButtonComponent } from "../task-action-button/task-action-button.component";
import { OpenVsCodeButtonComponent } from "../open-vscode-button/open-vscode-button.component";
import { OpenTerminalButtonComponent } from "../open-terminal-button/open-terminal-button.component";
import { StartAgentDropdownComponent } from "../start-agent-dropdown/start-agent-dropdown.component";
import { TaskStore } from "../../task.store";
import { LauncherService } from "../../../launcher/launcher.service";

@Component({
  selector: "app-task-view",
  standalone: true,
  imports: [
    CommonModule,
    FormsModule,
    TaskTerminalComponent,
    TaskDiffComponent,
    TaskActionButtonComponent,
    OpenVsCodeButtonComponent,
    OpenTerminalButtonComponent,
    StartAgentDropdownComponent,
  ],
  templateUrl: "./task-view.component.html",
  styleUrl: "./task-view.component.css",
})
export class TaskViewComponent {
  @Input() task: TaskSummary | null = null;
  @Input() baseRepo: BaseRepoInfo | null = null;
  activePane: "terminal" | "diff" = "terminal";
  @Output() startTask = new EventEmitter<{ taskId: string; agent: AgentKind }>();
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
  readonly agentKind = AgentKind;

  constructor(
    private readonly taskStore: TaskStore,
    private readonly launcher: LauncherService,
  ) {}

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
    return !!this.task &&
      ["IDLE", "AWAITING_APPROVAL", "WORKING"].includes(this.task.status);
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
    if (!this.task) {
      return;
    }
    if (!this.commitMessage.trim()) {
      this.commitError = "Commit message is required.";
      return;
    }
    this.commitError = "";
    try {
      await this.taskStore.commitTask(
        this.task.taskId,
        this.commitMessage.trim(),
        this.commitStageAll,
      );
      this.closeCommitModal();
    } catch (error: unknown) {
      this.commitError = this.describeError(error, "Unable to commit changes.");
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
    if (!this.task) {
      return;
    }
    this.pushError = "";
    try {
      await this.taskStore.pushTask(
        this.task.taskId,
        this.pushRemote.trim() || "origin",
        this.pushBranch.trim() || this.task.branchName,
        this.pushSetUpstream,
      );
      this.closePushModal();
    } catch (error: unknown) {
      this.pushError = this.describeError(error, "Unable to push changes.");
    }
  }

  onSelectBaseRepo(): void {
    this.selectBaseRepo.emit();
  }

  setActivePane(pane: "terminal" | "diff"): void {
    this.activePane = pane;
  }

  async openInExplorer(event: Event, path: string): Promise<void> {
    event.preventDefault();
    try {
      await this.launcher.openInExplorer(path);
    } catch (error) {
      console.error("Failed to open explorer", error);
    }
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
