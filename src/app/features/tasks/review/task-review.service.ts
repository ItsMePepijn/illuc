import { Injectable, NgZone } from "@angular/core";
import {
    ReviewComment,
    ReviewCommentStatus,
    ReviewTaskEntry,
    ReviewThread,
    ReviewStore,
} from "./models";
import { DiffLineType } from "../git/models";
import { tauriInvoke } from "../../../shared/tauri/tauri-zone";

const DEFAULT_REVIEW_STORE: ReviewStore = {
    version: 1,
    tasks: {},
};

@Injectable({
    providedIn: "root",
})
export class TaskReviewService {
    private userDisplayNamePromise: Promise<string> | null = null;

    constructor(private readonly zone: NgZone) {}

    async loadStore(worktreePath: string): Promise<ReviewStore> {
        const payload = await tauriInvoke<ReviewStore>(this.zone, "task_review_get", {
            req: { worktreePath },
        });
        return this.normalizeStore(payload);
    }

    async addComment(req: AddReviewCommentRequest): Promise<ReviewComment> {
        return tauriInvoke<ReviewComment>(this.zone, "task_review_add_comment", { req });
    }

    async editComment(req: EditReviewCommentRequest): Promise<ReviewComment> {
        return tauriInvoke<ReviewComment>(this.zone, "task_review_edit_comment", { req });
    }

    async deleteComment(
        req: DeleteReviewCommentRequest,
    ): Promise<DeleteReviewCommentResponse> {
        return tauriInvoke<DeleteReviewCommentResponse>(
            this.zone,
            "task_review_delete_comment",
            { req },
        );
    }

    async updateThreadStatus(
        req: UpdateReviewThreadStatusRequest,
    ): Promise<UpdateReviewThreadStatusResponse> {
        return tauriInvoke<UpdateReviewThreadStatusResponse>(
            this.zone,
            "task_review_update_thread_status",
            { req },
        );
    }

    getUserDisplayName(): Promise<string> {
        if (!this.userDisplayNamePromise) {
            this.userDisplayNamePromise = tauriInvoke<string>(
                this.zone,
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
