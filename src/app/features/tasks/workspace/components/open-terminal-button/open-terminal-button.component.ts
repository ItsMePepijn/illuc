import { CommonModule } from "@angular/common";
import { Component, Input } from "@angular/core";
import { LauncherService } from "../../../../launcher/launcher.service";
import { IconLoadingButtonComponent } from "../../../../../shared/components/icon-loading-button/icon-loading-button.component";
import { IconTerminalPanelComponent } from "../icon-terminal-panel/icon-terminal-panel.component";

@Component({
    selector: "app-open-terminal-button",
    standalone: true,
    imports: [CommonModule, IconLoadingButtonComponent, IconTerminalPanelComponent],
    templateUrl: "./open-terminal-button.component.html",
    styleUrl: "./open-terminal-button.component.css",
})
export class OpenTerminalButtonComponent {
    @Input() path: string | null = null;
    @Input() title = "Open terminal";
    @Input() ariaLabel = "Open terminal";
    isLoading = false;

    constructor(private readonly launcher: LauncherService) {}

    async handleClick(): Promise<void> {
        if (!this.path || this.isLoading) {
            return;
        }
        this.isLoading = true;
        try {
            await this.launcher.openTerminal(this.path);
        } catch (error) {
            console.error("Failed to open terminal", error);
        } finally {
            this.isLoading = false;
        }
    }
}
