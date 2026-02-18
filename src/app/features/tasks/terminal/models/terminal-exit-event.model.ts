import { TerminalKind } from "./terminal-kind.model";

export interface TerminalExitEvent {
    taskId: string;
    exitCode: number;
    kind: TerminalKind;
}
