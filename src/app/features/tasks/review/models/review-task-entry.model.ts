import { ReviewThread } from "./review-thread.model";

export interface ReviewTaskEntry {
    taskId: string;
    threads: ReviewThread[];
}
