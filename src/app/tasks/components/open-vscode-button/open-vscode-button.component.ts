import { CommonModule } from "@angular/common";
import { Component, Input } from "@angular/core";
import { LauncherService } from "../../../launcher/launcher.service";

@Component({
  selector: "app-open-vscode-button",
  standalone: true,
  imports: [CommonModule],
  templateUrl: "./open-vscode-button.component.html",
  styleUrl: "./open-vscode-button.component.css",
})
export class OpenVsCodeButtonComponent {
  @Input() path: string | null = null;
  @Input() title = "Open in VS Code";
  @Input() ariaLabel = "Open in VS Code";

  constructor(private readonly launcher: LauncherService) {}

  async handleClick(): Promise<void> {
    if (!this.path) {
      return;
    }
    try {
      await this.launcher.openInVsCode(this.path);
    } catch (error) {
      console.error("Failed to open VS Code", error);
    }
  }
}
