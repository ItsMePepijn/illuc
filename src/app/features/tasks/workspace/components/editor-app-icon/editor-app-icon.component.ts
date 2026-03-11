import { CommonModule } from "@angular/common";
import { Component, Input } from "@angular/core";
import { EditorIcon } from "../../../../launcher/models";

@Component({
    selector: "app-editor-app-icon",
    standalone: true,
    imports: [CommonModule],
    templateUrl: "./editor-app-icon.component.html",
    styleUrl: "./editor-app-icon.component.css",
})
export class EditorAppIconComponent {
    private _iconDataUrl: string | null = null;

    @Input() icon: EditorIcon | null = null;
    @Input() appName: string | null = null;
    failedToLoadIcon = false;

    @Input()
    set iconDataUrl(value: string | null) {
        this._iconDataUrl = value;
        this.failedToLoadIcon = false;
    }

    get iconDataUrl(): string | null {
        return this._iconDataUrl;
    }

    get shouldShowImage(): boolean {
        return Boolean(this.iconDataUrl) && !this.failedToLoadIcon;
    }

    get fallbackClass(): string {
        return `fallback-icon icon-${this.icon ?? "generic"}`;
    }

    get fallbackLabel(): string {
        switch (this.icon) {
            case "vscode":
            case "vscodeInsiders":
                return "{}";
            case "vscodium":
                return "[]";
            case "cursor":
                return "C";
            case "windsurf":
                return "W";
            case "zed":
                return "Z";
            case "sublimeText":
                return "S";
            case "notepadPlusPlus":
                return "N+";
            case "jetbrains":
                return "JB";
            case "generic":
            default:
                return (this.appName ?? "?").slice(0, 1).toUpperCase();
        }
    }

    handleImageError(): void {
        console.warn(
            `Launcher icon image failed for ${this.appName ?? this.icon ?? "unknown editor"}; falling back to default icon.`,
            {
                appName: this.appName,
                icon: this.icon,
                iconDataUrlPresent: Boolean(this.iconDataUrl),
            },
        );
        this.failedToLoadIcon = true;
    }
}
