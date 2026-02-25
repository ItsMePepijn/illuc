import { CommonModule } from "@angular/common";
import {
    ChangeDetectorRef,
    Component,
    Input,
    OnChanges,
    OnDestroy,
    SimpleChanges,
} from "@angular/core";

export type UsageWindowSnapshot = {
    used: number;
    limit: number;
    resetAt: string;
    windowDurationHours?: number | null;
    workingPeriods?: Array<{ startAt: string; endAt: string }>;
};

@Component({
    selector: "app-usage-window-rail",
    standalone: true,
    imports: [CommonModule],
    templateUrl: "./usage-window-rail.component.html",
    styleUrl: "./usage-window-rail.component.css",
})
export class UsageWindowRailComponent implements OnChanges, OnDestroy {
    @Input() taskId: string | null = null;
    @Input() windowHours = 7 * 24;
    @Input() fetchUsage: ((taskId: string) => Promise<UsageWindowSnapshot | null>) | null =
        null;

    usageRemainingRatio: number | null = null;
    usageSections = 0;
    usageNowPositionRatio: number | null = null;
    usageSectionTitles: string[] = [];

    private usageTimerId: number | null = null;

    constructor(private readonly cdr: ChangeDetectorRef) {}

    ngOnChanges(changes: SimpleChanges): void {
        if (changes["taskId"] || changes["fetchUsage"]) {
            this.connectUsage();
            return;
        }
        if (changes["windowHours"]) {
            this.recomputeNowPointer();
        }
    }

    ngOnDestroy(): void {
        this.stopUsagePolling();
    }

    usageSectionItems(): number[] {
        return Array.from({ length: this.usageSections }, (_value, index) => index);
    }

    usageSegmentFillPercent(index: number): number {
        if (this.usageRemainingRatio === null || this.usageSections <= 0) {
            return 0;
        }
        const filledSections = this.usageRemainingRatio * this.usageSections;
        const fromBottom = this.usageSections - 1 - index;
        if (filledSections >= fromBottom + 1) {
            return 100;
        }
        if (filledSections <= fromBottom) {
            return 0;
        }
        return (filledSections - fromBottom) * 100;
    }

    usageSectionTitle(index: number): string {
        return this.usageSectionTitles[index] ?? "";
    }

    usageNowPointerTitle(): string {
        return "Current time on your work schedule. Use this marker to compare where you are in your usage window versus how much quota is left. You can change your work schedule in the settings.";
    }

    usageRailTitle(): string {
        return "Usage pace rail. Each segment is one scheduled work period in your current usage window. You can change your work schedule in the settings.";
    }

    private connectUsage(): void {
        this.stopUsagePolling();
        if (!this.taskId || !this.fetchUsage) {
            this.usageRemainingRatio = null;
            this.usageSections = 0;
            this.usageNowPositionRatio = null;
            this.usageSectionTitles = [];
            return;
        }
        this.startUsagePolling();
    }

    private startUsagePolling(): void {
        this.stopUsagePolling();
        const taskId = this.taskId;
        if (!taskId) {
            return;
        }
        void this.refreshUsage(taskId);
        this.usageTimerId = window.setInterval(() => {
            void this.refreshUsage(taskId);
        }, 60_000);
    }

    private stopUsagePolling(): void {
        if (this.usageTimerId !== null) {
            window.clearInterval(this.usageTimerId);
            this.usageTimerId = null;
        }
    }

    private async refreshUsage(taskId: string): Promise<void> {
        const fetchUsage = this.fetchUsage;
        if (!fetchUsage) {
            return;
        }
        try {
            const usage = await fetchUsage(taskId);
            if (this.taskId !== taskId) {
                return;
            }
            if (!usage) {
                return;
            }
            this.applyUsage(usage);
        } catch {
            // Preserve last successful snapshot during transient backend errors.
        }
    }

    private applyUsage(usage: UsageWindowSnapshot | null): void {
        if (!usage || usage.limit <= 0) {
            this.usageRemainingRatio = null;
            this.usageSections = 0;
            this.usageNowPositionRatio = null;
            this.usageSectionTitles = [];
            this.cdr.markForCheck();
            return;
        }
        const remaining = Math.max(0, usage.limit - usage.used);
        this.usageRemainingRatio = Math.min(1, remaining / usage.limit);
        const reset = new Date(usage.resetAt);
        if (Number.isNaN(reset.getTime())) {
            this.usageSections = 1;
            this.usageNowPositionRatio = null;
            this.usageSectionTitles = [];
            this.cdr.markForCheck();
            return;
        }
        const endMs = reset.getTime();
        const startMs =
            endMs - (usage.windowDurationHours ?? this.windowHours) * 60 * 60 * 1000;
        const workingWindows = this.parseWorkingWindows(usage.workingPeriods, startMs, endMs);
        this.usageSections = Math.max(1, workingWindows.length);
        this.usageSectionTitles = this.buildUsageSectionTitles(workingWindows);
        this.usageNowPositionRatio = this.computeNowPositionRatio(workingWindows, Date.now());
        this.cdr.markForCheck();
    }

    private recomputeNowPointer(): void {
        if (this.usageSectionTitles.length === 0 || this.usageNowPositionRatio === null) {
            return;
        }
        // Keep UX stable: fetch fresh usage to recalculate using latest schedule/window settings.
        const taskId = this.taskId;
        if (!taskId) {
            return;
        }
        void this.refreshUsage(taskId);
    }

    private parseWorkingWindows(
        periods: Array<{ startAt: string; endAt: string }> | undefined,
        startMs: number,
        endMs: number,
    ): Array<{ startMs: number; endMs: number }> {
        if (
            !Array.isArray(periods) ||
            !Number.isFinite(startMs) ||
            !Number.isFinite(endMs) ||
            endMs <= startMs
        ) {
            return [];
        }

        const windows: Array<{ startMs: number; endMs: number }> = [];
        for (const period of periods) {
            const parsedStart = new Date(period.startAt).getTime();
            const parsedEnd = new Date(period.endAt).getTime();
            if (!Number.isFinite(parsedStart) || !Number.isFinite(parsedEnd)) {
                continue;
            }
            const overlapStart = Math.max(startMs, parsedStart);
            const overlapEnd = Math.min(endMs, parsedEnd);
            if (overlapEnd > overlapStart) {
                windows.push({ startMs: overlapStart, endMs: overlapEnd });
            }
        }
        return windows;
    }

    private computeNowPositionRatio(
        windows: Array<{ startMs: number; endMs: number }>,
        nowMs: number,
    ): number | null {
        if (windows.length === 0) {
            return null;
        }
        if (nowMs <= windows[0].startMs) {
            return 0;
        }
        const last = windows[windows.length - 1];
        if (nowMs >= last.endMs) {
            return 1;
        }

        let completed = 0;
        for (const window of windows) {
            if (nowMs < window.startMs) {
                return completed / windows.length;
            }
            if (nowMs >= window.endMs) {
                completed += 1;
                continue;
            }
            const span = window.endMs - window.startMs;
            if (span <= 0) {
                return completed / windows.length;
            }
            const inWindow = Math.max(0, nowMs - window.startMs);
            return (completed + inWindow / span) / windows.length;
        }
        return 1;
    }

    private buildUsageSectionTitles(
        windows: Array<{ startMs: number; endMs: number }>,
    ): string[] {
        const dateFmt = new Intl.DateTimeFormat(undefined, {
            weekday: "short",
            month: "short",
            day: "numeric",
        });
        const timeFmt = new Intl.DateTimeFormat(undefined, {
            hour: "numeric",
            minute: "2-digit",
        });
        return windows.map((window) => {
            const start = new Date(window.startMs);
            const end = new Date(window.endMs);
            return `${dateFmt.format(start)} ${timeFmt.format(start)}-${timeFmt.format(end)}`;
        });
    }
}
