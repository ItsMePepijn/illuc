export type ToolRowKind = "command" | "search" | "read" | "change" | "text";

export interface ToolRow {
    kind: ToolRowKind;
    label: string;
    value?: string | null;
    path?: string | null;
    added?: number | null;
    removed?: number | null;
}
