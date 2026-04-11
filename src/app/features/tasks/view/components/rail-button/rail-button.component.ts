import { CommonModule } from "@angular/common";
import { Component, EventEmitter, Input, Output, TemplateRef } from "@angular/core";
import { LoadingSpinnerComponent } from "../../../../../shared/components/loading-spinner/loading-spinner.component";

export type RailButtonVariant = "text" | "icon";

@Component({
    selector: "app-rail-button",
    standalone: true,
    imports: [CommonModule, LoadingSpinnerComponent],
    templateUrl: "./rail-button.component.html",
    styleUrl: "./rail-button.component.css",
})
export class RailButtonComponent {
    @Input() loading = false;
    @Input() disabled = false;
    @Input() buttonType: "button" | "submit" | "reset" = "button";
    @Input() ariaLabel?: string;
    @Input() ariaExpanded?: boolean | null;
    @Input() ariaHaspopup?: string | null;
    @Input() title?: string;
    @Input() icon?: TemplateRef<any>;
    @Input() dataAction?: string | null;
    @Input() buttonClass = "";
    @Input() stopPropagation = false;
    @Input() variant: RailButtonVariant = "text";
    @Output() action = new EventEmitter<MouseEvent>();

    get resolvedButtonClass(): string {
        const classes = ["rail-btn"];
        if (this.variant === "icon") {
            classes.push("rail-icon-btn");
        }
        if (this.buttonClass) {
            classes.push(this.buttonClass);
        }
        return classes.join(" ");
    }

    handleClick(event: MouseEvent): void {
        if (this.disabled || this.loading) {
            return;
        }
        if (this.stopPropagation) {
            event.stopPropagation();
        }
        this.action.emit(event);
    }
}
