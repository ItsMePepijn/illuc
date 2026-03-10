import { ToolRow } from "./tool-row.model";

export type MessagePresentationKind = "user" | "standard" | "reasoning" | "tool";
export type MessageTextFormat = "markdown" | "plain";

export interface MessagePresentation {
    kind: MessagePresentationKind;
    text?: string | null;
    textFormat?: MessageTextFormat | null;
    toolRows: ToolRow[];
    toolStatusLabel?: string | null;
    isToolRunning: boolean;
}
