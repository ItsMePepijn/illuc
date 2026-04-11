import { CommonModule } from "@angular/common";
import { Component, EventEmitter, Input, Output } from "@angular/core";
import { BaseRepoInfo } from "../../../models";

@Component({
    selector: "app-task-getting-started",
    standalone: true,
    imports: [CommonModule],
    templateUrl: "./task-getting-started.component.html",
    styleUrl: "./task-getting-started.component.css",
})
export class TaskGettingStartedComponent {
    @Input() baseRepo: BaseRepoInfo | null = null;
    @Output() frameMouseDown = new EventEmitter<MouseEvent>();

    onFrameMouseDown(event: MouseEvent): void {
        this.frameMouseDown.emit(event);
    }
}
