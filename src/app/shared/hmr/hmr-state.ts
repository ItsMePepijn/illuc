import type { CodexGuiStoreDevState } from "../../features/tasks/agents/codex-gui.store";
import type { TaskStoreDevState } from "../../features/tasks/task.store";

export type IllucHmrState = {
    taskStore?: TaskStoreDevState | null;
    codexGuiStore?: CodexGuiStoreDevState | null;
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
