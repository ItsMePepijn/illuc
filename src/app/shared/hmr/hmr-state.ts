import type { AgentChatStoreDevState } from "../../features/tasks/agent-chat/agent-chat.store";
import type { TaskStoreDevState } from "../../features/tasks/task.store";

export type IllucHmrState = {
    taskStore?: TaskStoreDevState | null;
    agentChatStore?: AgentChatStoreDevState | null;
};

declare global {
    var __ILLUC_HMR_STATE__: IllucHmrState | undefined;
}

export function readIllucHmrState(): IllucHmrState | null {
    return globalThis.__ILLUC_HMR_STATE__ ?? null;
}

export function writeIllucHmrState(state: IllucHmrState | null): void {
    if (state) {
        globalThis.__ILLUC_HMR_STATE__ = state;
        return;
    }
    delete globalThis.__ILLUC_HMR_STATE__;
}
