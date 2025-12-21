import { CommonModule } from "@angular/common";
import { Component, EventEmitter, Input, Output } from "@angular/core";
import { TaskSummary, BaseRepoInfo } from "../../task.models";
import { parseTitleParts, TitleParts } from "../../title.utils";
import { TaskTerminalComponent } from "../task-terminal/task-terminal.component";
import { TaskDiffComponent } from "../task-diff/task-diff.component";
import { TaskActionButtonComponent } from "../task-action-button/task-action-button.component";
import { OpenVsCodeButtonComponent } from "../open-vscode-button/open-vscode-button.component";
import { OpenTerminalButtonComponent } from "../open-terminal-button/open-terminal-button.component";

@Component({
  selector: "app-task-view",
  standalone: true,
  imports: [
    CommonModule,
    TaskTerminalComponent,
    TaskDiffComponent,
    TaskActionButtonComponent,
    OpenVsCodeButtonComponent,
    OpenTerminalButtonComponent,
  ],
  templateUrl: "./task-view.component.html",
  styleUrl: "./task-view.component.css",
})
export class TaskViewComponent {
  @Input() task: TaskSummary | null = null;
  @Input() baseRepo: BaseRepoInfo | null = null;
  activePane: "terminal" | "diff" = "terminal";
  @Output() startTask = new EventEmitter<string>();
  @Output() stopTask = new EventEmitter<string>();
  @Output() discardTask = new EventEmitter<string>();
  @Output() selectBaseRepo = new EventEmitter<void>();

  statusLabel(): string {
    return this.task?.status.replace(/_/g, " ") ?? "";
  }

  canStart(): boolean {
    return (
      !!this.task &&
      ["READY", "STOPPED", "COMPLETED", "FAILED"].includes(this.task.status)
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

  onStart(): void {
    if (this.task) {
      this.startTask.emit(this.task.taskId);
    }
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

  onSelectBaseRepo(): void {
    this.selectBaseRepo.emit();
  }

  setActivePane(pane: "terminal" | "diff"): void {
    this.activePane = pane;
  }
}
