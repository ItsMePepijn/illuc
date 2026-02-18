import { TimeTrackingEntry } from "./time-tracking-entry.model";

export interface TimeTrackingPayload {
    version: number;
    branches: Record<string, TimeTrackingEntry>;
}
