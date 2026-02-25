import { TokenUsage } from "./token-usage.model";

export interface TokenUsageEvent {
    taskId: string;
    usage: TokenUsage;
}
