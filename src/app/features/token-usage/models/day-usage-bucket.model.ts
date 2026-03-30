import { TokenUsageBreakdown } from "./token-usage-breakdown.model";

export interface DayUsageBucket {
    date: string;
    sessionCount: number;
    usage: TokenUsageBreakdown;
}
