import { CommonModule } from "@angular/common";
import {
    ChangeDetectionStrategy,
    Component,
    EventEmitter,
    Input,
    Output,
} from "@angular/core";

@Component({
    selector: "app-agent-chat-user-message",
    standalone: true,
    imports: [CommonModule],
    templateUrl: "./user-message.component.html",
    styleUrl: "./user-message.component.css",
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class UserMessageComponent {
    @Input() html = "";
    @Output() contentClick = new EventEmitter<MouseEvent>();
}
