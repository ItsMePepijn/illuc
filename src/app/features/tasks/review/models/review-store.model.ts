import { ReviewTaskEntry } from "./review-task-entry.model";

export interface ReviewStore {
    version: number;
    tasks: Record<string, ReviewTaskEntry>;
}
