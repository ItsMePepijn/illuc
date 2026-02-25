import { CommonModule } from "@angular/common";
import {
    ChangeDetectionStrategy,
    Component,
    EventEmitter,
    Input,
    Output,
} from "@angular/core";
import { ThrobberComponent } from "../../../throbber/throbber.component";

@Component({
    selector: "app-codex-gui-tool-message",
    standalone: true,
    imports: [CommonModule, ThrobberComponent],
    templateUrl: "./tool-message.component.html",
    styleUrl: "./tool-message.component.css",
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ToolMessageComponent {
    @Input() rowsHtml: string[] = [];
    @Input() isRunning = false;
    @Input() statusLabel = "";
    @Output() contentClick = new EventEmitter<MouseEvent>();
}
