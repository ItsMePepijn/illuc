import { CommonModule } from "@angular/common";
import { Component, Input } from "@angular/core";
import { LauncherService } from "../../../launcher/launcher.service";

@Component({
  selector: "app-open-terminal-button",
  standalone: true,
  imports: [CommonModule],
  templateUrl: "./open-terminal-button.component.html",
  styleUrl: "./open-terminal-button.component.css",
})
export class OpenTerminalButtonComponent {
  @Input() path: string | null = null;
  @Input() title = "Open terminal";
  @Input() ariaLabel = "Open terminal";

  constructor(private readonly launcher: LauncherService) {}

  async handleClick(): Promise<void> {
    if (!this.path) {
      return;
    }
    try {
      await this.launcher.openTerminal(this.path);
    } catch (error) {
      console.error("Failed to open terminal", error);
    }
  }
}
