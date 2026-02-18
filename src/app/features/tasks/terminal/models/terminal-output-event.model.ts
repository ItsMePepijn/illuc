import { TerminalKind } from "./terminal-kind.model";

export interface TerminalOutputEvent {
    taskId: string;
    data: string;
    kind: TerminalKind;
}
