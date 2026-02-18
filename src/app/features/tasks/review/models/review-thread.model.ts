import { DiffLineType } from "../../git/models/diff-line-type.model";
import { ReviewComment } from "./review-comment.model";
import { ReviewCommentStatus } from "./review-comment-status.model";

export interface ReviewThread {
    filePath: string;
    lineNumberOld?: number | null;
    lineNumberNew?: number | null;
    lineType: DiffLineType;
    status: ReviewCommentStatus;
    comments: ReviewComment[];
}
