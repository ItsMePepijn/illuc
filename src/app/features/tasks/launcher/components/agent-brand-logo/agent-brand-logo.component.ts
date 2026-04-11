import { CommonModule } from "@angular/common";
import { Component, Input } from "@angular/core";

@Component({
    selector: "app-agent-brand-logo",
    standalone: true,
    imports: [CommonModule],
    templateUrl: "./agent-brand-logo.component.html",
    styleUrl: "./agent-brand-logo.component.css",
})
export class AgentBrandLogoComponent {
    @Input({ required: true }) brand!: "openai" | "copilot" | "opencode";
}
