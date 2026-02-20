import { ChangeDetectorRef, Component, Input, NgZone } from "@angular/core";
import { LauncherService } from "../../../../launcher/launcher.service";
import { IconLoadingButtonComponent } from "../../../../../shared/components/icon-loading-button/icon-loading-button.component";
import { IconCogComponent } from "../icon-cog/icon-cog.component";

@Component({
    selector: "app-open-settings-button",
    standalone: true,
    imports: [IconLoadingButtonComponent, IconCogComponent],
    templateUrl: "./open-settings-button.component.html",
    styleUrl: "./open-settings-button.component.css",
})
export class OpenSettingsButtonComponent {
    @Input() buttonClass = "";
    isLoading = false;

    constructor(
        private readonly launcher: LauncherService,
        private readonly zone: NgZone,
        private readonly cdr: ChangeDetectorRef,
    ) {}

    async handleClick(): Promise<void> {
        if (this.isLoading) {
            return;
        }
        this.isLoading = true;
        try {
            await this.launcher.openSettingsInVsCode();
        } catch (error) {
            console.error("Failed to open settings.toml in VS Code", error);
        } finally {
            this.zone.run(() => {
                this.isLoading = false;
                this.cdr.markForCheck();
            });
        }
    }
}
