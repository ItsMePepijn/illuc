import { PlanStep } from "./plan-step.model";

export interface PlanEvent {
    taskId: string;
    explanation: string | null;
    plan: PlanStep[];
}
