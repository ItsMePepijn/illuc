import { CommonModule } from "@angular/common";
import {
    ChangeDetectionStrategy,
    Component,
    EventEmitter,
    Input,
    Output,
} from "@angular/core";

@Component({
    selector: "app-codex-gui-standard-message",
    standalone: true,
    imports: [CommonModule],
    templateUrl: "./standard-message.component.html",
    styleUrl: "./standard-message.component.css",
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class StandardMessageComponent {
    @Input() role: "assistant" | "system" = "assistant";
    @Input() streamingPlain = false;
    @Input() plainContent = "";
    @Input() html = "";
    @Output() contentClick = new EventEmitter<MouseEvent>();
}
