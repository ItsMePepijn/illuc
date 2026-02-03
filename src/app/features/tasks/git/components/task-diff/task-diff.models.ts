import { SafeHtml } from "@angular/platform-browser";
import { DiffLineType } from "../../../task.models";

export interface RenderedDiffLine {
    type: DiffLineType;
    html: SafeHtml;
}

export type DiffRowKind = "header" | "line" | "spacer" | "thread";

export interface RenderedDiffRow {
    kind: DiffRowKind;
    filePath: string;
    displayName?: string;
    status?: string;
    line?: RenderedDiffLine;
    lineNumberOld?: number | null;
    lineNumberNew?: number | null;
    threadTarget?: {
        filePath: string;
        lineNumberOld?: number | null;
        lineNumberNew?: number | null;
        lineType: DiffLineType;
    };
}
