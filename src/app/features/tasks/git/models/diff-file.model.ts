import { DiffLine } from "./diff-line.model";

export interface DiffFile {
    path: string;
    status: string;
    lines: DiffLine[];
}
