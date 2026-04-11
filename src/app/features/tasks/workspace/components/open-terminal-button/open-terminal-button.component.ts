import { CommonModule } from "@angular/common";
import { ChangeDetectorRef, Component, Input, NgZone } from "@angular/core";
import { LauncherService } from "../../../../launcher/launcher.service";
import { IconTerminalPanelComponent } from "../icon-terminal-panel/icon-terminal-panel.component";
import { RailButtonComponent } from "../../../view/components/rail-button/rail-button.component";

@Component({
    selector: "app-open-terminal-button",
    standalone: true,
    imports: [CommonModule, IconTerminalPanelComponent, RailButtonComponent],
    templateUrl: "./open-terminal-button.component.html",
    styleUrl: "./open-terminal-button.component.css",
})
export class OpenTerminalButtonComponent {
    @Input() path: string | null = null;
    @Input() title = "Open terminal";
    @Input() ariaLabel = "Open terminal";
    @Input() buttonClass = "";
    isLoading = false;

    constructor(
        private readonly launcher: LauncherService,
        private readonly zone: NgZone,
        private readonly cdr: ChangeDetectorRef,
    ) {}

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
            this.zone.run(() => {
                this.isLoading = false;
                this.cdr.markForCheck();
            });
        }
    }
}
