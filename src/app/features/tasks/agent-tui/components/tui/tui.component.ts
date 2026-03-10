import { CommonModule } from "@angular/common";
import { Component, Input } from "@angular/core";
import { TerminalSessionComponent } from "../../../terminal/components/terminal-session/terminal-session.component";

@Component({
    selector: "app-agent-tui",
    standalone: true,
    imports: [CommonModule, TerminalSessionComponent],
    templateUrl: "./tui.component.html",
    styleUrl: "./tui.component.css",
})
export class AgentTuiComponent {
    @Input() taskId: string | null = null;
    @Input() title = "Terminal";
    @Input() showToolbar = true;
}
