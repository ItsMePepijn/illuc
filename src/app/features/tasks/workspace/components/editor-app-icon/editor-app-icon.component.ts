import { CommonModule } from "@angular/common";
import { Component, Input } from "@angular/core";

@Component({
    selector: "app-editor-app-icon",
    standalone: true,
    imports: [CommonModule],
    templateUrl: "./editor-app-icon.component.html",
    styleUrl: "./editor-app-icon.component.css",
})
export class EditorAppIconComponent {
    @Input() iconDataUrl: string | null = null;
}
