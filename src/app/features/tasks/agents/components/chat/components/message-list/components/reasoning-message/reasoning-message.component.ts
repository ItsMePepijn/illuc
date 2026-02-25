import { CommonModule } from "@angular/common";
import {
    ChangeDetectionStrategy,
    Component,
    EventEmitter,
    Input,
    Output,
} from "@angular/core";

@Component({
    selector: "app-codex-gui-reasoning-message",
    standalone: true,
    imports: [CommonModule],
    templateUrl: "./reasoning-message.component.html",
    styleUrl: "./reasoning-message.component.css",
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ReasoningMessageComponent {
    @Input() streamingPlain = false;
    @Input() plainContent = "";
    @Input() html = "";
    @Output() contentClick = new EventEmitter<MouseEvent>();
}
