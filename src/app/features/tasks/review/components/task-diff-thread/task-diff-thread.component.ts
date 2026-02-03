import { CommonModule } from "@angular/common";
import {
    ChangeDetectionStrategy,
    Component,
    EventEmitter,
    Input,
    Output,
} from "@angular/core";
import { FormsModule } from "@angular/forms";
import { DomSanitizer, SafeHtml } from "@angular/platform-browser";
import { marked } from "marked";
import DOMPurify from "dompurify";
import {
    ReviewComment,
    ReviewCommentStatus,
} from "../../../task.models";

marked.setOptions({
    breaks: true,
    gfm: true,
});

type ReviewStatusOption = {
    value: ReviewCommentStatus;
    label: string;
};

@Component({
    selector: "app-task-diff-thread",
    standalone: true,
    imports: [CommonModule, FormsModule],
    templateUrl: "./task-diff-thread.component.html",
    styleUrl: "./task-diff-thread.component.css",
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TaskDiffThreadComponent {
    @Input() comments: ReviewComment[] = [];
    @Input() draft = "";
    @Input() canSubmit = false;
    @Input() threadStatus: ReviewCommentStatus = "active";
    @Input() isThreadStatusUpdating = false;
    @Input() editingCommentIds: ReadonlySet<string> = new Set();
    @Input() deletingCommentIds: ReadonlySet<string> = new Set();
    @Input() reviewStatusOptions: ReadonlyArray<ReviewStatusOption> = [];
    @Input() userDisplayName = "User";

    @Output() collapseThread = new EventEmitter<Event>();
    @Output() submitComment = new EventEmitter<Event>();
    @Output() draftChange = new EventEmitter<string>();
    @Output()
    statusChange = new EventEmitter<ReviewCommentStatus>();
    @Output()
    editComment = new EventEmitter<{
        comment: ReviewComment;
        body: string;
    }>();
    @Output() deleteComment = new EventEmitter<ReviewComment>();

    private readonly commentBodyCache = new Map<string, SafeHtml>();
    editingCommentId: string | null = null;
    editDraft = "";

    constructor(private readonly sanitizer: DomSanitizer) {}

    displayNameFor(author: string): string {
        if (author === "user") {
            return this.userDisplayName || "User";
        }
        return author;
    }

    renderCommentBody(comment: ReviewComment): SafeHtml {
        const normalizedBody = comment.body.replace(/\s+$/u, "");
        const key = `${comment.id}:${normalizedBody}`;
        const cached = this.commentBodyCache.get(key);
        if (cached) {
            return cached;
        }
        const rawHtml = marked.parse(normalizedBody) as string;
        const sanitized = DOMPurify.sanitize(rawHtml, {
            USE_PROFILES: { html: true },
        });
        const safe = this.sanitizer.bypassSecurityTrustHtml(sanitized);
        this.commentBodyCache.set(key, safe);
        return safe;
    }

    onStatusChange(status: ReviewCommentStatus): void {
        this.statusChange.emit(status);
    }

    startEditing(comment: ReviewComment): void {
        this.editingCommentId = comment.id;
        this.editDraft = comment.body;
    }

    cancelEditing(): void {
        this.editingCommentId = null;
        this.editDraft = "";
    }

    isEditing(comment: ReviewComment): boolean {
        return this.editingCommentId === comment.id;
    }

    canSaveEdit(comment: ReviewComment): boolean {
        const draft = this.editDraft.trim();
        return (
            draft.length > 0 &&
            draft !== comment.body.trim() &&
            !this.editingCommentIds.has(comment.id)
        );
    }

    onSaveEdit(comment: ReviewComment): void {
        if (!this.canSaveEdit(comment)) {
            return;
        }
        this.editComment.emit({
            comment,
            body: this.editDraft.trim(),
        });
        this.cancelEditing();
    }

    onDeleteComment(comment: ReviewComment): void {
        if (this.isEditing(comment)) {
            this.cancelEditing();
        }
        this.deleteComment.emit(comment);
    }
}
