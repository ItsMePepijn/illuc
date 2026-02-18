import { CommonModule } from "@angular/common";
import { Component, Input } from "@angular/core";
import { LauncherService } from "../../../../launcher/launcher.service";
import { IconLoadingButtonComponent } from "../../../../../shared/components/icon-loading-button/icon-loading-button.component";
import { IconCodeBracketsComponent } from "../icon-code-brackets/icon-code-brackets.component";

@Component({
    selector: "app-open-vscode-button",
    standalone: true,
    imports: [CommonModule, IconLoadingButtonComponent, IconCodeBracketsComponent],
    templateUrl: "./open-vscode-button.component.html",
    styleUrl: "./open-vscode-button.component.css",
})
export class OpenVsCodeButtonComponent {
    @Input() path: string | null = null;
    @Input() title = "Open in VS Code";
    @Input() ariaLabel = "Open in VS Code";
    isLoading = false;

    constructor(private readonly launcher: LauncherService) {}

    async handleClick(): Promise<void> {
        if (!this.path || this.isLoading) {
            return;
        }
        this.isLoading = true;
        try {
            await this.launcher.openInVsCode(this.path);
        } catch (error) {
            console.error("Failed to open VS Code", error);
        } finally {
            this.isLoading = false;
        }
    }
}
