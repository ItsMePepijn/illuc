import { AgentKind } from "./agent-kind.model";
import { TaskStatus } from "./task-status.model";

export interface TaskSummary {
    taskId: string;
    title: string;
    status: TaskStatus;
    agentKind: AgentKind;
    createdAt: string;
    startedAt?: string | null;
    endedAt?: string | null;
    worktreePath: string;
    branchName: string;
    baseBranch: string;
    baseRepoPath: string;
    baseCommit: string;
    exitCode?: number | null;
}
