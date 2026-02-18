import { Injectable, NgZone, signal } from "@angular/core";
import {
    TimeTrackingEntry,
    TimeTrackingPayload,
} from "./models";
import { tauriInvoke } from "../../shared/tauri/tauri-zone";

const DEFAULT_TRACKING: TimeTrackingPayload = {
    version: 1,
    branches: {},
};

@Injectable({
    providedIn: "root",
})
export class TaskTimeTrackingService {
    private readonly trackingSignal = signal<TimeTrackingPayload | null>(null);
    readonly tracking = this.trackingSignal.asReadonly();

    private baseRepoPath: string | null = null;
    private activeTask: TrackedTaskContext | null = null;
    private activeStartedAt: number | null = null;
    private flushTimer: ReturnType<typeof setInterval> | null = null;
    private readonly flushIntervalMs = 60_000;
    private flushQueue: Promise<void> = Promise.resolve();

    constructor(private readonly zone: NgZone) {
        window.addEventListener("beforeunload", this.handleUnload);
    }

    async syncContext(
        baseRepoPath: string | null,
        task: TrackedTaskContext | null,
    ): Promise<void> {
        if (baseRepoPath !== this.baseRepoPath) {
            await this.switchBaseRepo(baseRepoPath);
        }
        if (task?.taskId !== this.activeTask?.taskId) {
            await this.switchActiveTask(task);
        }
    }

    private handleUnload = (): void => {
        void this.flushActive();
    };

    private async switchBaseRepo(path: string | null): Promise<void> {
        const previousPath = this.baseRepoPath;
        if (previousPath && this.activeTask && this.activeStartedAt) {
            await this.flushActive(previousPath);
        }
        this.stopTimer();
        this.baseRepoPath = path;
        if (!path) {
            this.zone.run(() => {
                this.trackingSignal.set(null);
            });
            this.stopTimer();
            this.activeStartedAt = null;
            return;
        }
        this.zone.run(() => {
            this.trackingSignal.set(null);
        });
        await this.loadTracking(path);
        if (this.activeTask) {
            this.activeStartedAt = Date.now();
            this.ensureTimer();
        }
    }

    private async switchActiveTask(
        task: TrackedTaskContext | null,
    ): Promise<void> {
        const previousPath =
            this.activeTask?.baseRepoPath ?? this.baseRepoPath;
        await this.flushActive(previousPath);
        this.activeTask = task;
        const repoPath = this.baseRepoPath ?? task?.baseRepoPath ?? null;
        if (task && repoPath) {
            this.activeStartedAt = Date.now();
            this.ensureTimer();
            return;
        }
        this.activeStartedAt = null;
        this.stopTimer();
    }

    private async loadTracking(baseRepoPath: string): Promise<void> {
        try {
            const payload = await tauriInvoke<TimeTrackingPayload>(
                this.zone,
                "task_time_tracking_get",
                { req: { baseRepoPath } },
            );
            this.zone.run(() => {
                this.trackingSignal.set(this.normalizePayload(payload));
            });
        } catch (error) {
            console.error("Failed to load time tracking data", error);
            this.zone.run(() => {
                this.trackingSignal.set({ ...DEFAULT_TRACKING });
            });
        }
    }

    private normalizePayload(payload: TimeTrackingPayload): TimeTrackingPayload {
        if (!payload || typeof payload !== "object") {
            return { ...DEFAULT_TRACKING };
        }
        return {
            version: payload.version ?? DEFAULT_TRACKING.version,
            branches: payload.branches ?? {},
        };
    }

    private ensureTimer(): void {
        if (this.flushTimer || !this.activeTask || !this.baseRepoPath) {
            return;
        }
        this.zone.runOutsideAngular(() => {
            this.flushTimer = setInterval(() => {
                void this.flushActive();
            }, this.flushIntervalMs);
        });
    }

    private stopTimer(): void {
        if (this.flushTimer) {
            clearInterval(this.flushTimer);
            this.flushTimer = null;
        }
    }

    private flushActive(baseRepoOverride?: string | null): Promise<void> {
        this.flushQueue = this.flushQueue
            .then(() => this.performFlush(baseRepoOverride))
            .catch(() => undefined);
        return this.flushQueue;
    }

    private async performFlush(baseRepoOverride?: string | null): Promise<void> {
        const task = this.activeTask;
        const startedAt = this.activeStartedAt;
        const baseRepoPath =
            baseRepoOverride ?? this.baseRepoPath ?? task?.baseRepoPath ?? null;
        if (!task || !startedAt || !baseRepoPath) {
            return;
        }
        const now = Date.now();
        if (now <= startedAt) {
            this.activeStartedAt = now;
            return;
        }
        const branchName = task.branchName?.trim();
        if (!branchName) {
            this.activeStartedAt = now;
            return;
        }
        const days = this.splitByDate(startedAt, now);
        if (Object.keys(days).length === 0) {
            this.activeStartedAt = now;
            return;
        }
        try {
            await tauriInvoke<void>(this.zone, "task_time_tracking_record", {
                req: {
                    baseRepoPath,
                    branchName,
                    title: task.title,
                    days,
                },
            });
            this.zone.run(() => {
                this.applyLocalUpdate(branchName, task.title, days);
            });
        } catch (error) {
            console.error("Failed to persist time tracking", error);
        }
        if (this.activeTask?.taskId === task.taskId) {
            this.activeStartedAt = now;
        }
    }

    private applyLocalUpdate(
        branchName: string,
        title: string,
        days: Record<string, number>,
    ) {
        const current = this.trackingSignal() ?? { ...DEFAULT_TRACKING };
        const existing: TimeTrackingEntry = current.branches[branchName] ?? {
            branchName,
            title,
            byDate: {},
        };
        const updatedDays = { ...existing.byDate };
        for (const [day, seconds] of Object.entries(days)) {
            updatedDays[day] = (updatedDays[day] ?? 0) + seconds;
        }
        const updatedEntry: TimeTrackingEntry = {
            ...existing,
            title: title ?? existing.title,
            byDate: updatedDays,
        };
        this.trackingSignal.set({
            ...current,
            branches: {
                ...current.branches,
                [branchName]: updatedEntry,
            },
        });
    }

    private splitByDate(
        startMs: number,
        endMs: number,
    ): Record<string, number> {
        const result: Record<string, number> = {};
        let cursor = new Date(startMs);
        while (cursor.getTime() < endMs) {
            const dayStart = new Date(
                cursor.getFullYear(),
                cursor.getMonth(),
                cursor.getDate(),
            );
            const nextDay = new Date(dayStart);
            nextDay.setDate(dayStart.getDate() + 1);
            const sliceEnd = Math.min(endMs, nextDay.getTime());
            const seconds = Math.floor(
                (sliceEnd - cursor.getTime()) / 1000,
            );
            if (seconds > 0) {
                const key = this.toDateKey(dayStart);
                result[key] = (result[key] ?? 0) + seconds;
            }
            cursor = new Date(sliceEnd);
        }
        return result;
    }

    private toDateKey(date: Date): string {
        const year = date.getFullYear();
        const month = `${date.getMonth() + 1}`.padStart(2, "0");
        const day = `${date.getDate()}`.padStart(2, "0");
        return `${year}-${month}-${day}`;
    }
}

export interface TrackedTaskContext {
    taskId: string;
    branchName: string;
    title: string;
    baseRepoPath: string;
}
