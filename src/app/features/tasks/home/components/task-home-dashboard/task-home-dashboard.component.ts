import { CommonModule } from "@angular/common";
import {
    ChangeDetectionStrategy,
    Component,
    EventEmitter,
    Input,
    Output,
    computed,
    signal,
} from "@angular/core";
import { LoadingButtonComponent } from "../../../../../shared/components/loading-button/loading-button.component";
import { BaseRepoInfo, TaskSummary } from "../../../models";
import { TimeTrackingEntry } from "../../../../time-tracking/models";
import { TaskStore } from "../../../task.store";
import { TaskTimeTrackingService } from "../../../../time-tracking/task-time-tracking.service";

interface WeekdayCell {
    key: string;
    label: string;
    dateLabel: string;
}

interface TimeTrackingRow {
    branchName: string;
    title: string;
    byDate: Record<string, number>;
    total: number;
}

@Component({
    selector: "app-task-home-dashboard",
    standalone: true,
    imports: [CommonModule, LoadingButtonComponent],
    templateUrl: "./task-home-dashboard.component.html",
    styleUrl: "./task-home-dashboard.component.css",
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TaskHomeDashboardComponent {
    @Input() baseRepo: BaseRepoInfo | null = null;
    @Input() selectRepoLoading = false;
    @Input() selectRepoError = "";
    @Output() selectBaseRepo = new EventEmitter<void>();

    private readonly weekStartSignal = signal<Date>(
        this.startOfWeek(new Date()),
    );
    readonly weekDays = computed(() =>
        this.buildWeekDays(this.weekStartSignal()),
    );
    readonly weekRangeLabel = computed(() =>
        this.buildWeekRangeLabel(this.weekStartSignal()),
    );
    readonly rows = computed(() => this.buildRows());
    readonly weekTotalSeconds = computed(() =>
        this.rows().reduce((sum, row) => sum + row.total, 0),
    );

    constructor(
        private readonly taskStore: TaskStore,
        private readonly timeTracking: TaskTimeTrackingService,
    ) {
    }

    onSelectBaseRepo(): void {
        this.selectBaseRepo.emit();
    }

    trackByTaskId(_: number, row: TimeTrackingRow): string {
        return row.branchName;
    }

    formatDuration(seconds: number | undefined): string {
        if (!seconds) {
            return "-";
        }
        if (seconds < 60) {
            return "<1m";
        }
        const totalMinutes = Math.round(seconds / 60);
        const hours = Math.floor(totalMinutes / 60);
        const minutes = totalMinutes % 60;
        if (hours <= 0) {
            return `${minutes}m`;
        }
        if (minutes === 0) {
            return `${hours}h`;
        }
        return `${hours}h ${minutes}m`;
    }

    prevWeek(): void {
        const current = this.weekStartSignal();
        const next = new Date(current);
        next.setDate(current.getDate() - 7);
        this.weekStartSignal.set(this.startOfWeek(next));
    }

    nextWeek(): void {
        if (!this.canGoNext()) {
            return;
        }
        const current = this.weekStartSignal();
        const next = new Date(current);
        next.setDate(current.getDate() + 7);
        this.weekStartSignal.set(this.startOfWeek(next));
    }

    canGoNext(): boolean {
        const currentWeek = this.startOfWeek(new Date());
        return this.weekStartSignal().getTime() < currentWeek.getTime();
    }

    private buildRows(): TimeTrackingRow[] {
        const tracking = this.timeTracking.tracking();
        const tasks = this.taskStore.tasks();
        const tasksById = new Map(
            tasks.map((task) => [task.taskId, task]),
        );
        const entries = tracking
            ? (Object.values(tracking.branches ?? {}) as TimeTrackingEntry[])
            : [];
        return entries
            .map((entry) => this.buildRow(entry, tasksById))
            .filter((row): row is TimeTrackingRow => !!row)
            .sort((a, b) => b.total - a.total);
    }

    private buildRow(
        entry: {
            branchName: string;
            title?: string | null;
            byDate?: Record<string, number> | null;
        },
        tasksById: Map<string, TaskSummary>,
    ): TimeTrackingRow | null {
        const task =
            Array.from(tasksById.values()).find(
                (value) => value.branchName === entry.branchName,
            ) ?? null;
        const title = task?.title ?? entry.title ?? "Untitled task";
        const branchName = entry.branchName;
        let total = 0;
        const weekTotals: Record<string, number> = {};
        for (const day of this.weekDays()) {
            let seconds = 0;
            seconds += entry.byDate?.[day.key] ?? 0;
            weekTotals[day.key] = seconds;
            total += seconds;
        }
        if (total < 300) {
            return null;
        }
        return {
            title,
            branchName,
            byDate: weekTotals,
            total,
        };
    }

    private buildWeekDays(start: Date): WeekdayCell[] {
        return Array.from({ length: 7 }, (_, index) => {
            const day = new Date(start);
            day.setDate(start.getDate() + index);
            return {
                key: this.toDateKey(day),
                label: day.toLocaleDateString(undefined, {
                    weekday: "short",
                }),
                dateLabel: day.toLocaleDateString(undefined, {
                    day: "numeric",
                }),
            };
        });
    }

    private buildWeekRangeLabel(start: Date): string {
        const end = new Date(start);
        end.setDate(start.getDate() + 6);
        const startLabel = start.toLocaleDateString(undefined, {
            month: "short",
            day: "numeric",
        });
        const endLabel = end.toLocaleDateString(undefined, {
            month: "short",
            day: "numeric",
        });
        return `${startLabel} - ${endLabel}, ${start.getFullYear()}`;
    }

    private startOfWeek(date: Date): Date {
        const start = new Date(date);
        const day = start.getDay();
        const diff = (day + 6) % 7;
        start.setDate(start.getDate() - diff);
        start.setHours(0, 0, 0, 0);
        return start;
    }

    private toDateKey(date: Date): string {
        const year = date.getFullYear();
        const month = `${date.getMonth() + 1}`.padStart(2, "0");
        const day = `${date.getDate()}`.padStart(2, "0");
        return `${year}-${month}-${day}`;
    }
}
