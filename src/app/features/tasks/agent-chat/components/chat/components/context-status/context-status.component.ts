import { CommonModule } from "@angular/common";
import { ChangeDetectionStrategy, Component, Input } from "@angular/core";

@Component({
    selector: "app-agent-chat-context-status",
    standalone: true,
    imports: [CommonModule],
    templateUrl: "./context-status.component.html",
    styleUrl: "./context-status.component.css",
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ContextStatusComponent {
    @Input() limitWindowDurationMins: number | null = null;
    @Input() limitUsedPercent: number | null = null;
    @Input() contextWindowUsed: number | null = null;
    @Input() contextWindowSize: number | null = null;

    limitStatusParts():
        | {
              windowLabel: string;
              remainingPercent: number;
          }
        | null {
        const windowDurationMins = this.limitWindowDurationMins;
        const usedPercent = this.limitUsedPercent;
        if (
            windowDurationMins === null ||
            usedPercent === null ||
            !Number.isFinite(windowDurationMins) ||
            !Number.isFinite(usedPercent) ||
            windowDurationMins <= 0
        ) {
            return null;
        }
        return {
            windowLabel: this.formatWindowLabel(windowDurationMins),
            remainingPercent: Math.max(0, Math.round(100 - usedPercent)),
        };
    }

    contextStatusParts():
        | {
              used: string;
              size: string;
              remainingPercent: number;
          }
        | null {
        const used = this.contextWindowUsed;
        const size = this.contextWindowSize;
        if (
            used === null ||
            size === null ||
            !Number.isFinite(used) ||
            !Number.isFinite(size) ||
            size <= 0
        ) {
            return null;
        }
        return {
            used: this.formatCompactNumber(used),
            size: this.formatCompactNumber(size),
            remainingPercent: Math.max(
            0,
            Math.round(((size - used) / size) * 100),
            ),
        };
    }

    private formatCompactNumber(value: number): string {
        const absolute = Math.abs(value);
        if (absolute >= 1_000_000) {
            return `${this.formatCompactUnit(value / 1_000_000)}M`;
        }
        if (absolute >= 1_000) {
            return `${this.formatCompactUnit(value / 1_000)}K`;
        }
        return `${Math.round(value)}`;
    }

    private formatCompactUnit(value: number): string {
        const rounded =
            Math.abs(value) >= 10 ? Math.round(value) : Math.round(value * 10) / 10;
        return Number.isInteger(rounded) ? `${rounded}` : rounded.toFixed(1);
    }

    private formatWindowLabel(windowDurationMins: number): string {
        const hours = windowDurationMins / 60;
        if (Number.isInteger(hours) && hours >= 1) {
            return `${hours}h`;
        }
        if (windowDurationMins >= 60) {
            return `${Math.round(hours * 10) / 10}h`;
        }
        return `${Math.round(windowDurationMins)}m`;
    }
}
