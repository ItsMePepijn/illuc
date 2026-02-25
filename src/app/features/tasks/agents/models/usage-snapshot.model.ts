import { WorkingPeriod } from "./working-period.model";

export interface UsageSnapshot {
    used: number;
    limit: number;
    resetAt: string;
    windowDurationHours?: number | null;
    workingPeriods?: WorkingPeriod[];
}
