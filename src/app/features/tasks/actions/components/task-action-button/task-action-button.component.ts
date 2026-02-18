import { CommonModule } from "@angular/common";
import { Component, EventEmitter, Input, Output } from "@angular/core";
import { LoadingButtonComponent } from "../../../../../shared/components/loading-button/loading-button.component";

export type TaskActionButtonType = "stop" | "discard" | "commit" | "push";
export type TaskActionButtonVariant = "icon" | "text";

@Component({
    selector: "app-task-action-button",
    standalone: true,
    imports: [CommonModule, LoadingButtonComponent],
    templateUrl: "./task-action-button.component.html",
    styleUrl: "./task-action-button.component.css",
})
export class TaskActionButtonComponent {
    @Input({ required: true }) type: TaskActionButtonType = "stop";
    @Input() variant?: TaskActionButtonVariant;
    @Input() disabled = false;
    @Input() loading = false;
    @Input({ required: true }) title = "";
    @Input() ariaLabel?: string;
    @Input() stopPropagation = false;
    @Output() action = new EventEmitter<void>();

    handleClick(event: MouseEvent): void {
        if (this.disabled || this.loading) {
            return;
        }
        if (this.stopPropagation) {
            event.stopPropagation();
        }
        this.action.emit();
    }

    get computedAriaLabel(): string {
        if (this.ariaLabel) {
            return this.ariaLabel;
        }
        return this.title;
    }

    get buttonClass(): string {
        const classes = ["action-btn"];
        if (this.resolvedVariant === "icon") {
            classes.unshift("icon-btn");
        }
        if (this.resolvedVariant === "text") {
            classes.push("action-text-btn");
        }
        if (this.type === "discard") {
            classes.push("warn");
        }
        return classes.join(" ");
    }

    get resolvedVariant(): TaskActionButtonVariant {
        if (this.variant) {
            return this.variant;
        }
        return this.type === "commit" || this.type === "push"
            ? "text"
            : "icon";
    }
}
