import { CommonModule } from "@angular/common";
import { Component, EventEmitter, Input, Output } from "@angular/core";
import { LoadingButtonComponent } from "../loading-button/loading-button.component";

@Component({
    selector: "app-icon-loading-button",
    standalone: true,
    imports: [CommonModule, LoadingButtonComponent],
    templateUrl: "./icon-loading-button.component.html",
    styleUrl: "./icon-loading-button.component.css",
})
export class IconLoadingButtonComponent {
    @Input() loading = false;
    @Input() disabled = false;
    @Input() buttonType: "button" | "submit" | "reset" = "button";
    @Input() ariaLabel?: string;
    @Input() title?: string;
    @Input() dataAction?: string | null;
    @Input() buttonClass = "";
    @Input() stopPropagation = false;
    @Output() action = new EventEmitter<MouseEvent>();

    get resolvedButtonClass(): string {
        return this.buttonClass
            ? `icon-btn ${this.buttonClass}`.trim()
            : "icon-btn";
    }
}
