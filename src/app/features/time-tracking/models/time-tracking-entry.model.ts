export interface TimeTrackingEntry {
    branchName: string;
    title?: string | null;
    byDate: Record<string, number>;
}
