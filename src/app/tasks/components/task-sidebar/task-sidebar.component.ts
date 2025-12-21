import { CommonModule } from "@angular/common";
import { Component, EventEmitter, Input, Output } from "@angular/core";
import { TaskStatus, TaskSummary, BaseRepoInfo } from "../../task.models";
import { parseTitleParts, TitleParts } from "../../title.utils";
import { TaskActionButtonComponent } from "../task-action-button/task-action-button.component";
import { OpenVsCodeButtonComponent } from "../open-vscode-button/open-vscode-button.component";
import { OpenTerminalButtonComponent } from "../open-terminal-button/open-terminal-button.component";

@Component({
  selector: "app-task-sidebar",
  standalone: true,
  imports: [
    CommonModule,
    TaskActionButtonComponent,
    OpenVsCodeButtonComponent,
    OpenTerminalButtonComponent,
  ],
  templateUrl: "./task-sidebar.component.html",
  styleUrl: "./task-sidebar.component.css",
})
export class TaskSidebarComponent {
  @Input({ required: true }) tasks: TaskSummary[] | null = [];
  @Input() selectedTaskId: string | null = null;
  @Input() baseRepo: BaseRepoInfo | null = null;
  @Output() selectTask = new EventEmitter<string>();
  @Output() startTask = new EventEmitter<string>();
  @Output() stopTask = new EventEmitter<string>();
  @Output() discardTask = new EventEmitter<string>();
  @Output() createTask = new EventEmitter<void>();

  trackById(_: number, task: TaskSummary): string {
    return task.taskId;
  }

  onSelect(taskId: string): void {
    this.selectTask.emit(taskId);
  }

  onStart(taskId: string): void {
    this.startTask.emit(taskId);
  }

  onStop(taskId: string): void {
    this.stopTask.emit(taskId);
  }

  onDiscard(taskId: string): void {
    this.discardTask.emit(taskId);
  }

  statusLabel(status: TaskStatus): string {
    return status.replace(/_/g, " ");
  }

  canStart(status: TaskStatus): boolean {
    return (
      status === "STOPPED" ||
      status === "FAILED" ||
      status === "COMPLETED"
    );
  }

  isRunning(status: TaskStatus): boolean {
    return (
      status === "IDLE" ||
      status === "AWAITING_APPROVAL" ||
      status === "WORKING"
    );
  }

  titleParts(title: string): TitleParts {
    return parseTitleParts(title);
  }
}
