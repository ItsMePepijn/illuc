import { CommonModule } from "@angular/common";
import { Component, EventEmitter, HostListener, Input, Output } from "@angular/core";
import { AgentKind } from "../../task.models";

@Component({
  selector: "app-start-agent-dropdown",
  standalone: true,
  imports: [CommonModule],
  templateUrl: "./start-agent-dropdown.component.html",
  styleUrl: "./start-agent-dropdown.component.css",
})
export class StartAgentDropdownComponent {
  @Input() disabled = false;
  @Output() start = new EventEmitter<AgentKind>();

  menuOpen = false;
  readonly options = [
    { kind: AgentKind.Codex, label: "Codex" },
    { kind: AgentKind.Copilot, label: "Copilot" },
  ];

  toggleMenu(event: MouseEvent): void {
    event.stopPropagation();
    if (this.disabled) {
      this.menuOpen = false;
      return;
    }
    this.menuOpen = !this.menuOpen;
  }

  choose(kind: AgentKind, event: MouseEvent): void {
    event.stopPropagation();
    this.menuOpen = false;
    this.start.emit(kind);
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
