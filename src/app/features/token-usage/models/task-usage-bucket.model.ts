import { TokenUsageBreakdown } from "./token-usage-breakdown.model";

export interface TaskUsageBucket {
    key: string;
    label: string;
    subtitle: string;
    path: string;
    isWorkspace: boolean;
    sessionCount: number;
    lastActiveAt: string;
    usage: TokenUsageBreakdown;
}
