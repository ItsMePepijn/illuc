import { CommonModule } from "@angular/common";
import {
    ChangeDetectionStrategy,
    Component,
    EventEmitter,
    Input,
    Output,
} from "@angular/core";
import { AgentChatSlashCommand } from "../../../../../../agent-chat.store";

@Component({
    selector: "app-agent-chat-slash-menu",
    standalone: true,
    imports: [CommonModule],
    templateUrl: "./slash-menu.component.html",
    styleUrl: "./slash-menu.component.css",
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class SlashMenuComponent {
    @Input() commands: AgentChatSlashCommand[] = [];
    @Input() selectedIndex = 0;

    @Output() commandSelected = new EventEmitter<AgentChatSlashCommand>();

    onSelect(command: AgentChatSlashCommand, event: MouseEvent): void {
        event.preventDefault();
        event.stopPropagation();
        this.commandSelected.emit(command);
    }

    slashCommandLabel(command: AgentChatSlashCommand): string {
        return `/${command.name}`;
    }
}
