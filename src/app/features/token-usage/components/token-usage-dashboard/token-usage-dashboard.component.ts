import { CommonModule } from "@angular/common";
import {
    ChangeDetectionStrategy,
    Component,
    HostListener,
    OnInit,
    computed,
    signal,
} from "@angular/core";
import { TokenUsageService } from "../../token-usage.service";
import {
    DayUsageBucket,
    TaskUsageBucket,
    TokenUsageBreakdown,
} from "../../models";
import {
    UsageBarChartComponent,
    UsageChartBar,
    UsageMetric,
} from "../usage-bar-chart/usage-bar-chart.component";

const EMPTY_SCOPE = {
    totals: {
        inputTokens: 0,
        cachedInputTokens: 0,
        outputTokens: 0,
        totalTokens: 0,
        inputCost: 0,
        cachedInputCost: 0,
        outputCost: 0,
        totalCost: 0,
    } satisfies TokenUsageBreakdown,
    sessionCount: 0,
    byDay: [] as DayUsageBucket[],
    byMonthSessionCounts: {} as Record<string, number>,
};

@Component({
    selector: "app-token-usage-dashboard",
    standalone: true,
    imports: [CommonModule, UsageBarChartComponent],
    templateUrl: "./token-usage-dashboard.component.html",
    styleUrl: "./token-usage-dashboard.component.css",
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TokenUsageDashboardComponent implements OnInit {
    metric = signal<UsageMetric>("costs");
    scope = signal<"global" | "workspace">("workspace");
    selectedMonth = signal(this.currentMonthKey());
    monthMenuOpen = signal(false);
    readonly usage = computed(() => this.tokenUsage.usage());
    readonly loading = computed(() => this.tokenUsage.loading());
    readonly monthOptions = computed(() => {
        const options = new Map<string, string>();
        const allBuckets = [
            ...(this.usage()?.scopes?.global.byDay ?? []),
            ...(this.usage()?.scopes?.workspace.byDay ?? []),
        ];
        for (const bucket of allBuckets) {
            options.set(bucket.date.slice(0, 7), this.formatMonthLabel(bucket.date));
        }
        const currentMonth = this.currentMonthKey();
        if (!options.has(currentMonth)) {
            options.set(currentMonth, this.formatMonthLabel(`${currentMonth}-01`));
        }
        return [...options.entries()]
            .sort((left, right) => right[0].localeCompare(left[0]))
            .map(([value, label]) => ({ value, label }));
    });
    readonly totals = computed(() =>
        this.monthBuckets().reduce(
            (aggregate, bucket) => ({
                inputTokens: aggregate.inputTokens + bucket.usage.inputTokens,
                cachedInputTokens:
                    aggregate.cachedInputTokens + bucket.usage.cachedInputTokens,
                outputTokens: aggregate.outputTokens + bucket.usage.outputTokens,
                totalTokens: aggregate.totalTokens + bucket.usage.totalTokens,
                inputCost: aggregate.inputCost + bucket.usage.inputCost,
                cachedInputCost:
                    aggregate.cachedInputCost + bucket.usage.cachedInputCost,
                outputCost: aggregate.outputCost + bucket.usage.outputCost,
                totalCost: aggregate.totalCost + bucket.usage.totalCost,
            }),
            this.emptyBreakdown(),
        ),
    );
    readonly dayBars = computed(() => {
        const monthKey = this.selectedMonth();
        const buckets = new Map(
            this.monthBuckets().map((bucket) => [bucket.date, bucket]),
        );
        return this.daysInMonth(monthKey).map((date) =>
            this.mapDayBucket(
                buckets.get(date) ?? {
                    date,
                    sessionCount: 0,
                    usage: this.emptyBreakdown(),
                },
            ),
        );
    });
    readonly taskBars = computed(() =>
        (this.usage()?.byTask ?? []).map((bucket) => this.mapTaskBucket(bucket)),
    );
    readonly totalSessions = computed(() =>
        this.activeScope().byMonthSessionCounts?.[this.selectedMonth()] ?? 0,
    );

    constructor(
        public readonly tokenUsage: TokenUsageService,
    ) {}

    ngOnInit(): void {
        void this.tokenUsage.refresh();
    }

    metricValue(usage: TokenUsageBreakdown): string {
        if (this.metric() === "costs") {
            const roundedValue =
                usage.totalCost < 1 ? usage.totalCost : Math.round(usage.totalCost);
            return new Intl.NumberFormat(undefined, {
                style: "currency",
                currency: this.usage()?.currency ?? "USD",
                minimumFractionDigits: roundedValue < 1 ? 2 : 0,
                maximumFractionDigits: roundedValue < 1 ? 2 : 0,
            }).format(roundedValue);
        }
        return this.formatTokenValue(usage.totalTokens);
    }

    formatSegmentValue(
        segment: "input" | "cached" | "output",
        usage: TokenUsageBreakdown,
    ): string {
        if (this.metric() === "costs") {
            const value =
                segment === "input"
                    ? usage.inputCost
                    : segment === "cached"
                      ? usage.cachedInputCost
                      : usage.outputCost;
            const roundedValue = value < 1 ? value : Math.round(value);
            return new Intl.NumberFormat(undefined, {
                style: "currency",
                currency: this.usage()?.currency ?? "USD",
                minimumFractionDigits: roundedValue < 1 ? 2 : 0,
                maximumFractionDigits: roundedValue < 1 ? 2 : 0,
            }).format(roundedValue);
        }
        const value =
            segment === "input"
                ? usage.inputTokens
                : segment === "cached"
                  ? usage.cachedInputTokens
                  : usage.outputTokens;
        return this.formatTokenValue(value);
    }

    setMetric(metric: UsageMetric): void {
        this.metric.set(metric);
    }

    setScope(scope: "global" | "workspace"): void {
        this.scope.set(scope);
    }

    setSelectedMonth(month: string): void {
        this.selectedMonth.set(month);
        this.monthMenuOpen.set(false);
    }

    toggleMonthMenu(event: MouseEvent): void {
        event.preventDefault();
        event.stopPropagation();
        this.monthMenuOpen.update((isOpen) => !isOpen);
    }

    selectedMonthLabel(): string {
        const selected = this.monthOptions().find(
            (option) => option.value === this.selectedMonth(),
        );
        return selected?.label ?? this.formatMonthLabel(`${this.selectedMonth()}-01`);
    }

    @HostListener("document:click")
    closeMonthMenu(): void {
        this.monthMenuOpen.set(false);
    }

    private activeScope() {
        return this.scope() === "global"
            ? this.usage()?.scopes?.global ?? EMPTY_SCOPE
            : this.usage()?.scopes?.workspace ?? EMPTY_SCOPE;
    }

    private monthBuckets(): DayUsageBucket[] {
        const monthKey = this.selectedMonth();
        return (this.activeScope().byDay ?? []).filter((bucket) =>
            bucket.date.startsWith(`${monthKey}-`),
        );
    }

    private mapDayBucket(bucket: DayUsageBucket): UsageChartBar {
        const date = new Date(`${bucket.date}T00:00:00`);
        return {
            key: bucket.date,
            label: date.toLocaleDateString(undefined, {
                month: "short",
                day: "numeric",
            }),
            shortLabel: date.toLocaleDateString(undefined, {
                day: "numeric",
            }),
            subtitle: date.toLocaleDateString(undefined, {
                weekday: "short",
            }),
            sessionCount: bucket.sessionCount,
            usage: bucket.usage,
        };
    }

    private daysInMonth(monthKey: string): string[] {
        const [yearText, monthText] = monthKey.split("-");
        const year = Number(yearText);
        const month = Number(monthText);
        if (!Number.isFinite(year) || !Number.isFinite(month)) {
            return [];
        }
        const days = new Date(year, month, 0).getDate();
        return Array.from({ length: days }, (_, index) => {
            const day = `${index + 1}`.padStart(2, "0");
            return `${monthKey}-${day}`;
        });
    }

    private currentMonthKey(): string {
        const now = new Date();
        return [
            now.getFullYear(),
            `${now.getMonth() + 1}`.padStart(2, "0"),
        ].join("-");
    }

    private formatMonthLabel(dateText: string): string {
        const date = new Date(`${dateText.slice(0, 7)}-01T00:00:00`);
        return date.toLocaleDateString(undefined, {
            month: "long",
            year: "numeric",
        });
    }

    private mapTaskBucket(bucket: TaskUsageBucket): UsageChartBar {
        return {
            key: bucket.key,
            label: bucket.label,
            shortLabel: this.truncate(bucket.label, 14),
            subtitle: bucket.subtitle,
            sessionCount: bucket.sessionCount,
            usage: bucket.usage,
        };
    }

    private truncate(value: string, maxLength: number): string {
        if (value.length <= maxLength) {
            return value;
        }
        return `${value.slice(0, Math.max(0, maxLength - 1))}…`;
    }

    private formatTokenValue(value: number): string {
        if (value >= 1_000_000) {
            return `${(value / 1_000_000).toFixed(value >= 10_000_000 ? 0 : 1)}M`;
        }
        if (value >= 1_000) {
            return `${(value / 1_000).toFixed(value >= 10_000 ? 0 : 1)}k`;
        }
        return Math.round(value).toString();
    }

    private emptyBreakdown(): TokenUsageBreakdown {
        return {
            inputTokens: 0,
            cachedInputTokens: 0,
            outputTokens: 0,
            totalTokens: 0,
            inputCost: 0,
            cachedInputCost: 0,
            outputCost: 0,
            totalCost: 0,
        };
    }
}
