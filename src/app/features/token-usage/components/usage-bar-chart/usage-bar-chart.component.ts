import { CommonModule } from "@angular/common";
import {
    ChangeDetectionStrategy,
    Component,
    EventEmitter,
    HostListener,
    HostBinding,
    Input,
    Output,
} from "@angular/core";
import { TokenUsageBreakdown } from "../../models";

export type UsageMetric = "tokens" | "costs";
export type UsageChartOrientation = "vertical" | "horizontal";

export interface UsageChartBar {
    key: string;
    label: string;
    shortLabel: string;
    subtitle: string;
    sessionCount: number;
    usage: TokenUsageBreakdown;
}

export interface UsageChartHeaderSelectOption {
    value: string;
    label: string;
}

@Component({
    selector: "app-usage-bar-chart",
    standalone: true,
    imports: [CommonModule],
    templateUrl: "./usage-bar-chart.component.html",
    styleUrl: "./usage-bar-chart.component.css",
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class UsageBarChartComponent {
    @Input({ required: true }) title = "";
    @Input({ required: true }) description = "";
    @Input({ required: true }) bars: UsageChartBar[] = [];
    @Input() metric: UsageMetric = "tokens";
    @Input() currency = "USD";
    @Input() compactBars = false;
    @Input() orientation: UsageChartOrientation = "vertical";
    @Input() fillAvailableHeight = false;
    @Input() scrollContent = false;
    @Input() headerSelectOptions: UsageChartHeaderSelectOption[] = [];
    @Input() headerSelectValue = "";
    @Input() headerSelectAriaLabel = "Chart selection";
    @Output() readonly headerSelectValueChange = new EventEmitter<string>();
    headerSelectOpen = false;

    @HostBinding("class.fill-available-height")
    get fillAvailableHeightClass(): boolean {
        return this.fillAvailableHeight;
    }

    maxTotal(): number {
        return Math.max(
            ...this.bars.map((bar) => this.totalValue(bar.usage)),
            0,
        );
    }

    totalValue(usage: TokenUsageBreakdown): number {
        return this.metric === "costs" ? usage.totalCost : usage.totalTokens;
    }

    totalHeight(usage: TokenUsageBreakdown): number {
        const max = this.maxTotal();
        if (max <= 0) {
            return 0;
        }
        return (this.totalValue(usage) / max) * 100;
    }

    totalWidth(usage: TokenUsageBreakdown): number {
        const max = this.maxTotal();
        if (max <= 0) {
            return 0;
        }
        return (this.totalValue(usage) / max) * 100;
    }

    segmentShare(
        usage: TokenUsageBreakdown,
        segment: "input" | "cached" | "output",
    ): number {
        const total = this.totalValue(usage);
        if (total <= 0) {
            return 0;
        }
        return (this.segmentValue(usage, segment) / total) * 100;
    }

    segmentValue(
        usage: TokenUsageBreakdown,
        segment: "input" | "cached" | "output",
    ): number {
        if (this.metric === "costs") {
            if (segment === "input") {
                return usage.inputCost;
            }
            if (segment === "cached") {
                return usage.cachedInputCost;
            }
            return usage.outputCost;
        }
        if (segment === "input") {
            return usage.inputTokens;
        }
        if (segment === "cached") {
            return usage.cachedInputTokens;
        }
        return usage.outputTokens;
    }

    formatValue(value: number): string {
        if (this.metric === "costs") {
            const roundedValue = value < 1 ? value : Math.round(value);
            return new Intl.NumberFormat(undefined, {
                style: "currency",
                currency: this.currency,
                minimumFractionDigits: roundedValue < 1 ? 2 : 0,
                maximumFractionDigits: roundedValue < 1 ? 2 : 0,
            }).format(roundedValue);
        }
        if (value >= 1_000_000) {
            return `${(value / 1_000_000).toFixed(value >= 10_000_000 ? 0 : 1)}M`;
        }
        if (value >= 1_000) {
            return `${(value / 1_000).toFixed(value >= 10_000 ? 0 : 1)}k`;
        }
        return Math.round(value).toString();
    }

    tooltip(bar: UsageChartBar): string {
        const total = this.formatValue(this.totalValue(bar.usage));
        const input = this.formatValue(this.segmentValue(bar.usage, "input"));
        const cached = this.formatValue(this.segmentValue(bar.usage, "cached"));
        const output = this.formatValue(this.segmentValue(bar.usage, "output"));
        return `${bar.label}
${bar.subtitle}
Total: ${total}
Input: ${input}
Cached: ${cached}
Output: ${output}
Sessions: ${bar.sessionCount}`;
    }

    gridTemplateColumns(): string {
        const minWidth = this.compactBars ? 32 : 88;
        return `repeat(${this.bars.length}, minmax(${minWidth}px, 1fr))`;
    }

    selectedHeaderOptionLabel(): string {
        const selected = this.headerSelectOptions.find(
            (option) => option.value === this.headerSelectValue,
        );
        return selected?.label ?? this.headerSelectOptions[0]?.label ?? "";
    }

    toggleHeaderSelect(event: MouseEvent): void {
        if (this.headerSelectOptions.length === 0) {
            return;
        }
        event.preventDefault();
        event.stopPropagation();
        this.headerSelectOpen = !this.headerSelectOpen;
    }

    selectHeaderOption(value: string, event: MouseEvent): void {
        event.preventDefault();
        event.stopPropagation();
        this.headerSelectValueChange.emit(value);
        this.headerSelectOpen = false;
    }

    @HostListener("document:click")
    closeHeaderSelect(): void {
        this.headerSelectOpen = false;
    }
}
