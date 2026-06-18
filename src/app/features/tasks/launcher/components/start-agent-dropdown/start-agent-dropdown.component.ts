import { CommonModule } from "@angular/common";
import {
    Component,
    computed,
    EventEmitter,
    Input,
    Output,
} from "@angular/core";
import { AgentKind, AgentKindAvailability } from "../../../models";
import { TaskStore } from "../../../task.store";
import { AgentBrandLogoComponent } from "../agent-brand-logo/agent-brand-logo.component";

type AgentTile = {
    kind: AgentKind;
    label: string;
    subtitle: string;
    brand: "openai" | "copilot" | "opencode" | "claude";
    title: string;
    installed: boolean;
};

const DEFAULT_UNAVAILABLE_TITLE =
    "Not installed on this system, but supported by illuc.";

@Component({
    selector: "app-start-agent-dropdown",
    standalone: true,
    imports: [CommonModule, AgentBrandLogoComponent],
    templateUrl: "./start-agent-dropdown.component.html",
    styleUrl: "./start-agent-dropdown.component.css",
})
export class StartAgentDropdownComponent {
    @Input() disabled = false;
    @Input() loading = false;
    @Output() start = new EventEmitter<AgentKind>();

    readonly tiles = computed(() =>
        this.buildTiles(this.taskStore.agentKinds() ?? []),
    );

    constructor(private readonly taskStore: TaskStore) {}

    choose(kind: AgentKind): void {
        const tile = this.tiles().find((candidate) => candidate.kind === kind);
        if (!tile || this.disabled || this.loading || !tile.installed) {
            return;
        }
        this.start.emit(kind);
    }

    private buildTiles(
        availability: AgentKindAvailability[] = [],
    ): AgentTile[] {
        const availabilityByKind = new Map(
            availability.map((entry) => [entry.kind, entry]),
        );
        return [
            this.buildTile(
                AgentKind.CodexGui,
                "Codex",
                "Graphical Interface",
                "openai",
                availabilityByKind,
            ),
            this.buildTile(
                AgentKind.CopilotGui,
                "Copilot",
                "Graphical Interface",
                "copilot",
                availabilityByKind,
            ),
            this.buildTile(
                AgentKind.Codex,
                "Codex",
                "Terminal Interface",
                "openai",
                availabilityByKind,
            ),
            this.buildTile(
                AgentKind.Copilot,
                "Copilot",
                "Terminal Interface",
                "copilot",
                availabilityByKind,
            ),
            this.buildTile(
                AgentKind.ClaudeCode,
                "Claude Code",
                "Terminal Interface",
                "claude",
                availabilityByKind,
            ),
            this.buildTile(
                AgentKind.OpenCode,
                "OpenCode",
                "Terminal Interface",
                "opencode",
                availabilityByKind,
            ),
        ];
    }

    private buildTile(
        kind: AgentKind,
        label: string,
        subtitle: string,
        brand: AgentTile["brand"],
        availabilityByKind: Map<AgentKind, AgentKindAvailability>,
    ): AgentTile {
        const availability = availabilityByKind.get(kind);
        const installed = availability?.installed ?? false;
        return {
            kind,
            label,
            subtitle,
            brand,
            installed,
            title: installed ? `${label} ${subtitle}` : DEFAULT_UNAVAILABLE_TITLE,
        };
    }
}
