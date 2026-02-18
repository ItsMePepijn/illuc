import { DiffFile } from "./diff-file.model";

export interface DiffPayload {
    taskId: string;
    files: DiffFile[];
}
