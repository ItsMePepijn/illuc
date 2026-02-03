import { Injectable } from "@angular/core";
import { invoke } from "@tauri-apps/api/core";
import {
    DiffLineType,
    ReviewComment,
    ReviewCommentStatus,
    ReviewTaskEntry,
    ReviewThread,
    ReviewStore,
} from "../task.models";

const DEFAULT_REVIEW_STORE: ReviewStore = {
    version: 1,
    tasks: {},
};

@Injectable({
    providedIn: "root",
})
export class TaskReviewService {
    private userDisplayNamePromise: Promise<string> | null = null;

    async loadStore(worktreePath: string): Promise<ReviewStore> {
        const payload = await invoke<ReviewStore>("task_review_get", {
            req: { worktreePath },
        });
        return this.normalizeStore(payload);
    }

    async addComment(req: AddReviewCommentRequest): Promise<ReviewComment> {
        return invoke<ReviewComment>("task_review_add_comment", { req });
    }

    async editComment(req: EditReviewCommentRequest): Promise<ReviewComment> {
        return invoke<ReviewComment>("task_review_edit_comment", { req });
    }

    async deleteComment(
        req: DeleteReviewCommentRequest,
    ): Promise<DeleteReviewCommentResponse> {
        return invoke<DeleteReviewCommentResponse>("task_review_delete_comment", {
            req,
        });
    }

    async updateThreadStatus(
        req: UpdateReviewThreadStatusRequest,
    ): Promise<UpdateReviewThreadStatusResponse> {
        return invoke<UpdateReviewThreadStatusResponse>(
            "task_review_update_thread_status",
            {
            req,
            },
        );
    }

    getUserDisplayName(): Promise<string> {
        if (!this.userDisplayNamePromise) {
            this.userDisplayNamePromise = invoke<string>(
                "task_review_get_user_display_name",
            ).catch((error) => {
                console.error("Failed to fetch user display name", error);
                return "User";
            });
        }
        return this.userDisplayNamePromise;
    }

    private normalizeStore(payload: ReviewStore | null | undefined): ReviewStore {
        if (!payload || typeof payload !== "object") {
            return { ...DEFAULT_REVIEW_STORE };
        }
        const normalizedTasks: ReviewStore["tasks"] = {};
        for (const [taskId, entry] of Object.entries(payload.tasks ?? {})) {
            normalizedTasks[taskId] = this.normalizeTaskEntry(taskId, entry);
        }
        return {
            version: payload.version ?? DEFAULT_REVIEW_STORE.version,
            tasks: normalizedTasks,
        };
    }

    private normalizeTaskEntry(
        taskId: string,
        entry: ReviewTaskEntry | null | undefined,
    ): ReviewTaskEntry {
        return {
            taskId: entry?.taskId ?? taskId,
            threads: (entry?.threads ?? []).map((thread) =>
                this.normalizeThread(thread),
            ),
        };
    }

    private normalizeThread(thread: ReviewThread): ReviewThread {
        return {
            filePath: thread.filePath,
            lineNumberOld: thread.lineNumberOld ?? null,
            lineNumberNew: thread.lineNumberNew ?? null,
            lineType: thread.lineType,
            status: thread.status ?? "active",
            comments: thread.comments ?? [],
        };
    }
}

export interface AddReviewCommentRequest {
    worktreePath: string;
    taskId: string;
    filePath: string;
    lineNumberOld?: number | null;
    lineNumberNew?: number | null;
    lineType: DiffLineType;
    body: string;
}

export interface UpdateReviewThreadStatusRequest {
    worktreePath: string;
    taskId: string;
    filePath: string;
    lineNumberOld?: number | null;
    lineNumberNew?: number | null;
    status: ReviewCommentStatus;
}

export interface UpdateReviewThreadStatusResponse {
    threadKey: string;
    status: ReviewCommentStatus;
}

export interface EditReviewCommentRequest {
    worktreePath: string;
    taskId: string;
    filePath: string;
    lineNumberOld?: number | null;
    lineNumberNew?: number | null;
    commentId: string;
    body: string;
}

export interface DeleteReviewCommentRequest {
    worktreePath: string;
    taskId: string;
    filePath: string;
    lineNumberOld?: number | null;
    lineNumberNew?: number | null;
    commentId: string;
}

export interface DeleteReviewCommentResponse {
    commentId: string;
}
