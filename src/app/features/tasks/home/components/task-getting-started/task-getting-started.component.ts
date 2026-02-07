import { CommonModule } from "@angular/common";
import { Component, Input } from "@angular/core";
import { BaseRepoInfo } from "../../../task.models";

@Component({
    selector: "app-task-getting-started",
    standalone: true,
    imports: [CommonModule],
    templateUrl: "./task-getting-started.component.html",
    styleUrl: "./task-getting-started.component.css",
})
export class TaskGettingStartedComponent {
    @Input() baseRepo: BaseRepoInfo | null = null;
}
