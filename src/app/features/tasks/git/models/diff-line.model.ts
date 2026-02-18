import { DiffLineType } from "./diff-line-type.model";

export interface DiffLine {
    type: DiffLineType;
    content: string;
    lineNumberOld?: number | null;
    lineNumberNew?: number | null;
}
