import { CommonModule } from "@angular/common";
import {
    ChangeDetectionStrategy,
    Component,
    EventEmitter,
    Input,
    Output,
} from "@angular/core";
import { ToolRow } from "../../../../../../codex-gui/models";
import { ThrobberComponent } from "../../../throbber/throbber.component";
import { toDisplayPath } from "../../codex-gui-message-list-renderer";

@Component({
    selector: "app-codex-gui-tool-message",
    standalone: true,
    imports: [CommonModule, ThrobberComponent],
    templateUrl: "./tool-message.component.html",
    styleUrl: "./tool-message.component.css",
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ToolMessageComponent {
    @Input() rows: ToolRow[] = [];
    @Input() isRunning = false;
    @Input() statusLabel = "";
    @Input() stripPathPrefix = "";
    @Output() contentClick = new EventEmitter<MouseEvent>();

    rowValue(row: ToolRow): string {
        if (row.path) {
            return this.toDisplayPath(row.path);
        }
        return row.value?.trim() ?? "";
    }

    showLabel(row: ToolRow): boolean {
        return row.kind !== "command";
    }

    rowHref(row: ToolRow): string | null {
        if (!row.path) {
            return null;
        }
        return `#diff:${encodeURIComponent(row.path)}`;
    }

    private toDisplayPath(path: string): string {
        return toDisplayPath(path, this.stripPathPrefix);
    }
}
