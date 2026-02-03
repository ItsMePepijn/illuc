import { Injectable } from "@angular/core";
import { invoke } from "@tauri-apps/api/core";
import {
    DiffLineType,
    ReviewComment,
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
        return {
            version: payload.version ?? DEFAULT_REVIEW_STORE.version,
            tasks: payload.tasks ?? {},
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
