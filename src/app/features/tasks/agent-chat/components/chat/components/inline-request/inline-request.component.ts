import { CommonModule } from "@angular/common";
import { Component, EventEmitter, Input, Output } from "@angular/core";
import { FormsModule } from "@angular/forms";
import { AgentChatRequestState } from "../../../../agent-chat.store";

@Component({
    selector: "app-agent-chat-inline-request",
    standalone: true,
    imports: [CommonModule, FormsModule],
    templateUrl: "./inline-request.component.html",
    styleUrl: "./inline-request.component.css",
})
export class InlineRequestComponent {
    @Input() request: AgentChatRequestState | null = null;
    @Input() requestAnswers: Record<string, string[]> = {};

    @Output() decisionSelected = new EventEmitter<string>();
    @Output() answerChanged = new EventEmitter<{
        questionId: string;
        value: string;
    }>();
    @Output() submitAnswers = new EventEmitter<void>();

    requestQuestionValue(questionId: string): string {
        return (this.requestAnswers[questionId] ?? []).join("\n");
    }

    onDecision(decision: string): void {
        this.decisionSelected.emit(decision);
    }

    onAnswerChanged(questionId: string, value: string): void {
        this.answerChanged.emit({ questionId, value });
    }

    onSubmitAnswers(): void {
        this.submitAnswers.emit();
    }

    decisionLabel(decision: string): string {
        return decision
            .trim()
            .split(/\s+/)
            .filter((part) => part.length > 0)
            .map((part) => part.charAt(0).toUpperCase() + part.slice(1).toLowerCase())
            .join(" ");
    }
}
