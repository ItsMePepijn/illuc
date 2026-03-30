import { DayUsageBucket } from "./day-usage-bucket.model";
import { TaskUsageBucket } from "./task-usage-bucket.model";
import { TokenUsageBreakdown } from "./token-usage-breakdown.model";

export interface TokenUsageScopePayload {
    totals: TokenUsageBreakdown;
    sessionCount: number;
    byDay: DayUsageBucket[];
    byMonthSessionCounts: Record<string, number>;
}

export interface TokenUsagePayload {
    version: number;
    currency: string;
    pricingVersion: number;
    pricingSourceUrl: string;
    pricingPublishedAt: string;
    note: string;
    scopes: {
        global: TokenUsageScopePayload;
        workspace: TokenUsageScopePayload;
    };
    byTask: TaskUsageBucket[];
    unknownPricedModels: string[];
}
