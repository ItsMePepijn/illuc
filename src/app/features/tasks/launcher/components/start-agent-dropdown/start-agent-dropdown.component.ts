import { CommonModule } from "@angular/common";
import {
    Component,
    EventEmitter,
    HostListener,
    Input,
    Output,
    OnChanges,
    SimpleChanges,
} from "@angular/core";
import { AgentKind } from "../../../models";
import { LoadingButtonComponent } from "../../../../../shared/components/loading-button/loading-button.component";

@Component({
    selector: "app-start-agent-dropdown",
    standalone: true,
    imports: [CommonModule, LoadingButtonComponent],
    templateUrl: "./start-agent-dropdown.component.html",
    styleUrl: "./start-agent-dropdown.component.css",
})
export class StartAgentDropdownComponent implements OnChanges {
    @Input() disabled = false;
    @Input() loading = false;
    @Output() start = new EventEmitter<AgentKind>();

    menuOpen = false;
    readonly options = [
        { kind: AgentKind.CodexGui, label: "Codex" },
        { kind: AgentKind.CopilotGui, label: "Copilot" },
        { kind: AgentKind.Codex, label: "Codex" },
        { kind: AgentKind.Copilot, label: "Copilot" },
        { kind: AgentKind.OpenCode, label: "OpenCode" },
    ];

    isGuiAgent(kind: AgentKind): boolean {
        return kind === AgentKind.CodexGui || kind === AgentKind.CopilotGui;
    }

    isCodexAgent(kind: AgentKind): boolean {
        return kind === AgentKind.CodexGui || kind === AgentKind.Codex;
    }

    isCopilotAgent(kind: AgentKind): boolean {
        return kind === AgentKind.CopilotGui || kind === AgentKind.Copilot;
    }

    toggleMenu(event: MouseEvent): void {
        event.stopPropagation();
        if (this.disabled || this.loading) {
            this.menuOpen = false;
            return;
        }
        this.menuOpen = !this.menuOpen;
    }

    choose(kind: AgentKind, event: MouseEvent): void {
        event.stopPropagation();
        if (this.disabled || this.loading) {
            return;
        }
        this.menuOpen = false;
        this.start.emit(kind);
    }

    ngOnChanges(changes: SimpleChanges): void {
        if (changes["loading"] && this.loading) {
            this.menuOpen = false;
        }
    }

    @HostListener("document:click")
    handleDocumentClick(): void {
        this.menuOpen = false;
    }

    @HostListener("document:keydown.escape", ["$event"])
    handleEscape(event: Event): void {
        if (!this.menuOpen) {
            return;
        }
        event.preventDefault();
        this.menuOpen = false;
    }
}
