import { RequestQuestion } from "./request-question.model";

export interface RequestEvent {
    taskId: string;
    requestId: string | null;
    kind: string;
    itemId: string | null;
    approvalId: string | null;
    command: string | null;
    cwd: string | null;
    reason: string | null;
    networkHost: string | null;
    networkProtocol: string | null;
    additionalReadRoots: string[];
    additionalWriteRoots: string[];
    additionalNetwork: boolean;
    availableDecisions: string[];
    proposedExecPolicy: string[];
    proposedNetworkPolicy: string[];
    grantRoot: string | null;
    questions: RequestQuestion[];
}
