import { CommonModule } from "@angular/common";
import {
    ChangeDetectionStrategy,
    Component,
    EventEmitter,
    Input,
    Output,
} from "@angular/core";
import { ToolRow } from "../../../../../../models";
import { ThrobberComponent } from "../../../throbber/throbber.component";
import { toDisplayPath } from "../../agent-chat-message-list-renderer";

@Component({
    selector: "app-agent-chat-tool-message",
    standalone: true,
    imports: [CommonModule, ThrobberComponent],
    templateUrl: "./tool-message.component.html",
    styleUrl: "./tool-message.component.css",
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ToolMessageComponent {
    @Input() rows: ToolRow[] = [];
    @Input() groupTitle = "";
    @Input() groupMeta = "";
    @Input() groupCount = 0;
    @Input() isRunning = false;
    @Input() statusLabel = "";
    @Input() stripPathPrefix = "";
    @Output() contentClick = new EventEmitter<MouseEvent>();

    private readonly groupedRowLimit = 5;

    get isGrouped(): boolean {
        return this.groupTitle.trim().length > 0;
    }

    get visibleRows(): ToolRow[] {
        if (!this.isGrouped) {
            return this.rows;
        }
        return this.rows.slice(0, this.groupedRowLimit);
    }

    get hiddenRowCount(): number {
        if (!this.isGrouped) {
            return 0;
        }
        return Math.max(0, this.rows.length - this.visibleRows.length);
    }

    rowMarker(index: number): string {
        if (!this.isGrouped) {
            return ">_";
        }
        const isLastVisibleRow = index === this.visibleRows.length - 1;
        const isCurrentRunningRow = this.isRunning && this.hiddenRowCount === 0 && isLastVisibleRow;
        return isCurrentRunningRow ? "◼" : "✔";
    }

    rowValue(row: ToolRow): string {
        if (row.path) {
            return this.toDisplayPath(row.path);
        }
        return row.value?.trim() ?? "";
    }

    showLabel(row: ToolRow): boolean {
        return row.kind !== "command";
    }

    showRowLabel(row: ToolRow): boolean {
        return this.isGrouped ? row.kind === "text" : this.showLabel(row);
    }

    isLiveCommandOutput(row: ToolRow): boolean {
        return this.isRunning && row.kind === "command" && this.rowValue(row).includes("\n");
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
