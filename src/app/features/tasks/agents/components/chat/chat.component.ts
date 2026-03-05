import { CommonModule } from "@angular/common";
import {
    ChangeDetectionStrategy,
    ChangeDetectorRef,
    Component,
    Input,
    NgZone,
    OnChanges,
    OnDestroy,
    SimpleChanges,
} from "@angular/core";
import { FormsModule } from "@angular/forms";
import { Subscription } from "rxjs";
import { LimitStatus, Message } from "../../models";
import {
    CodexGuiActivityState,
    CodexGuiStore,
    CodexGuiPlanState,
    CodexGuiRequestState,
    CodexGuiTokenUsageState,
} from "../../codex-gui.store";
import { TaskStore } from "../../../task.store";
import {
    ComposerComponent,
    CodexGuiModelOption,
} from "./components/composer/composer.component";
import { InlineRequestComponent } from "./components/inline-request/inline-request.component";
import { MessageListComponent } from "./components/message-list/message-list.component";
import {
    UsageWindowRailComponent,
    UsageWindowSnapshot,
} from "../usage-window-rail/usage-window-rail.component";

const LIMIT_STATUS_POLL_INTERVAL_MS = 60_000;
const LIMIT_STATUS_BOOTSTRAP_POLL_INTERVAL_MS = 2_500;
const LIMIT_STATUS_BOOTSTRAP_MAX_ATTEMPTS = 12;

@Component({
    selector: "app-codex-gui-chat",
    standalone: true,
    imports: [
        CommonModule,
        FormsModule,
        MessageListComponent,
        InlineRequestComponent,
        ComposerComponent,
        UsageWindowRailComponent,
    ],
    templateUrl: "./chat.component.html",
    styleUrl: "./chat.component.css",
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ChatComponent implements OnChanges, OnDestroy {
    @Input() taskId: string | null = null;
    @Input() isWorking = false;
    @Input() stripPathPrefix = "";
    @Input() isActive = true;

    messages: Message[] = [];
    messagesHydrated = false;
    activity: CodexGuiActivityState | null = null;
    plan: CodexGuiPlanState | null = null;
    tokenUsage: CodexGuiTokenUsageState | null = null;
    limitStatus: LimitStatus | null = null;
    pendingRequest: CodexGuiRequestState | null = null;
    prompt = "";
    sending = false;
    selectedModel = "";
    selectedEffort = "";
    modelOptions: CodexGuiModelOption[] = [];
    effortOptions: CodexGuiModelOption[] = [];
    requestAnswers: Record<string, string[]> = {};

    readonly fetchUsageForRail = async (
        taskId: string,
    ): Promise<UsageWindowSnapshot | null> => this.codexGuiStore.getUsage(taskId);

    private messageSubscription?: Subscription;
    private hydrationSubscription?: Subscription;
    private activitySubscription?: Subscription;
    private planSubscription?: Subscription;
    private tokenUsageSubscription?: Subscription;
    private requestSubscription?: Subscription;
    private limitStatusTimerId: number | null = null;
    private limitStatusBootstrapTimerId: number | null = null;
    private limitStatusBootstrapAttempts = 0;

    constructor(
        private readonly taskStore: TaskStore,
        private readonly codexGuiStore: CodexGuiStore,
        private readonly zone: NgZone,
        private readonly cdr: ChangeDetectorRef,
    ) {}

    ngOnChanges(changes: SimpleChanges): void {
        if (changes["taskId"]) {
            this.connectMessages();
        }
    }

    ngOnDestroy(): void {
        this.messageSubscription?.unsubscribe();
        this.hydrationSubscription?.unsubscribe();
        this.activitySubscription?.unsubscribe();
        this.planSubscription?.unsubscribe();
        this.tokenUsageSubscription?.unsubscribe();
        this.requestSubscription?.unsubscribe();
        this.stopLimitStatusPolling();
    }

    async send(): Promise<void> {
        if (!this.taskId || this.sending) {
            return;
        }
        const text = this.prompt.trim();
        if (!text) {
            return;
        }
        this.prompt = "";
        this.sending = true;
        try {
            await this.codexGuiStore.sendMessage(
                this.taskId,
                text,
                this.selectedModel,
                this.selectedEffort,
            );
        } finally {
            this.sending = false;
            this.cdr.markForCheck();
        }
    }

    onModelChange(model: string): void {
        if (!this.taskId) {
            return;
        }
        this.selectedModel = model;
        this.codexGuiStore.setModel(this.taskId, model);
        this.refreshEffortOptions();
        this.cdr.markForCheck();
    }

    onEffortChange(effort: string): void {
        if (!this.taskId) {
            return;
        }
        this.selectedEffort = effort;
        this.codexGuiStore.setEffort(this.taskId, effort);
        this.cdr.markForCheck();
    }

    async stop(): Promise<void> {
        if (!this.taskId) {
            return;
        }
        try {
            await this.codexGuiStore.interruptTask(this.taskId);
        } finally {
            this.cdr.markForCheck();
        }
    }

    requestQuestionValue(questionId: string): string {
        return (this.requestAnswers[questionId] ?? []).join("\n");
    }

    setRequestQuestionValue(questionId: string, value: string): void {
        this.requestAnswers = {
            ...this.requestAnswers,
            [questionId]: value
                .split("\n")
                .map((item) => item.trim())
                .filter((item) => item.length > 0),
        };
        this.cdr.markForCheck();
    }

    async approve(decision: string): Promise<void> {
        if (!this.taskId || !this.pendingRequest?.requestId) {
            return;
        }
        await this.codexGuiStore.respondToRequest(
            this.taskId,
            this.pendingRequest.requestId,
            { decision },
        );
        this.requestAnswers = {};
    }

    async submitUserInput(): Promise<void> {
        if (!this.taskId || !this.pendingRequest?.requestId) {
            return;
        }
        const answers: Record<string, { answers: string[] }> = {};
        for (const question of this.pendingRequest.questions) {
            const values = this.requestAnswers[question.id] ?? [];
            answers[question.id] = { answers: values };
        }
        await this.codexGuiStore.respondToRequest(
            this.taskId,
            this.pendingRequest.requestId,
            { answers },
        );
        this.requestAnswers = {};
    }

    trackPlanStep(_index: number, step: { step: string; status: string }): string {
        return `${step.status}:${step.step}`;
    }

    onDiffFileRequested(filePath: string): void {
        if (!this.taskId) {
            return;
        }
        this.taskStore.requestDiffJump(this.taskId, filePath);
    }

    private connectMessages(): void {
        this.messageSubscription?.unsubscribe();
        this.hydrationSubscription?.unsubscribe();
        this.activitySubscription?.unsubscribe();
        this.planSubscription?.unsubscribe();
        this.tokenUsageSubscription?.unsubscribe();
        this.requestSubscription?.unsubscribe();
        this.stopLimitStatusPolling();
        if (!this.taskId) {
            this.messages = [];
            this.messagesHydrated = true;
            this.activity = null;
            this.plan = null;
            this.tokenUsage = null;
            this.limitStatus = null;
            this.pendingRequest = null;
            this.requestAnswers = {};
            this.prompt = "";
            this.selectedModel = "";
            this.selectedEffort = "";
            this.modelOptions = [];
            this.effortOptions = [];
            this.cdr.markForCheck();
            return;
        }

        this.messages = this.codexGuiStore.getMessages(this.taskId);
        this.messagesHydrated =
            this.messages.length > 0 ||
            this.codexGuiStore.isHydrated(this.taskId);
        this.activity = this.codexGuiStore.getActivity(this.taskId);
        this.plan = this.codexGuiStore.getPlan(this.taskId);
        this.tokenUsage = this.codexGuiStore.getTokenUsage(this.taskId);
        this.limitStatus = null;
        this.pendingRequest = this.codexGuiStore.getRequest(this.taskId);
        this.selectedModel = this.codexGuiStore.getModel(this.taskId);
        this.selectedEffort = this.codexGuiStore.getEffort(this.taskId);
        this.messageSubscription = this.codexGuiStore
            .messages$(this.taskId)
            .subscribe((messages) => {
                this.zone.run(() => {
                    this.messages = messages;
                    if (messages.length > 0) {
                        this.messagesHydrated = true;
                    }
                    this.cdr.detectChanges();
                });
            });
        this.hydrationSubscription = this.codexGuiStore
            .hydrated$(this.taskId)
            .subscribe((hydrated) => {
                this.zone.run(() => {
                    this.messagesHydrated = hydrated || this.messages.length > 0;
                    this.cdr.detectChanges();
                });
            });
        this.activitySubscription = this.codexGuiStore
            .activity$(this.taskId)
            .subscribe((activity) => {
                this.zone.run(() => {
                    this.activity = activity;
                    this.cdr.detectChanges();
                });
            });
        this.planSubscription = this.codexGuiStore
            .plan$(this.taskId)
            .subscribe((plan) => {
                this.zone.run(() => {
                    this.plan = plan;
                    this.cdr.detectChanges();
                });
            });
        this.tokenUsageSubscription = this.codexGuiStore
            .tokenUsage$(this.taskId)
            .subscribe((tokenUsage) => {
                this.zone.run(() => {
                    this.tokenUsage = tokenUsage;
                    this.cdr.detectChanges();
                });
            });
        this.requestSubscription = this.codexGuiStore
            .request$(this.taskId)
            .subscribe((request) => {
                this.zone.run(() => {
                    this.pendingRequest = request;
                    this.requestAnswers = this.buildInitialRequestAnswers(request);
                    this.cdr.detectChanges();
                });
            });

        void this.loadAvailableModels();
        this.startLimitStatusPolling(this.taskId);
        this.cdr.markForCheck();
    }

    private buildInitialRequestAnswers(
        request: CodexGuiRequestState | null,
    ): Record<string, string[]> {
        if (!request || request.kind !== "userInput") {
            return {};
        }
        const answers: Record<string, string[]> = {};
        for (const question of request.questions) {
            const firstOption = question.options[0]?.label?.trim();
            answers[question.id] = firstOption ? [firstOption] : [];
        }
        return answers;
    }

    private async loadAvailableModels(): Promise<void> {
        const taskId = this.taskId;
        if (!taskId) {
            return;
        }
        try {
            let models: string[] = [];
            for (let attempt = 0; attempt < 5; attempt += 1) {
                models = await this.codexGuiStore.getAvailableModels(taskId);
                if (models.length > 1 || attempt === 4) {
                    break;
                }
                await new Promise((resolve) => window.setTimeout(resolve, 300));
            }
            if (this.taskId !== taskId || models.length === 0) {
                return;
            }
            this.selectedModel = this.codexGuiStore.getModel(taskId);
            this.selectedEffort = this.codexGuiStore.getEffort(taskId);
            this.modelOptions = models.map((model) => ({
                value: model,
                label: model.replaceAll("-", " "),
            }));
            if (this.selectedModel && !models.includes(this.selectedModel)) {
                this.selectedModel = "";
                this.codexGuiStore.setModel(taskId, "");
            }
            this.refreshEffortOptions();
            this.cdr.markForCheck();
        } catch {
            // keep fallback
        }
    }

    private startLimitStatusPolling(taskId: string): void {
        this.stopLimitStatusPolling();
        void this.refreshLimitStatus(taskId);
        this.startLimitStatusBootstrapPolling(taskId);
        this.limitStatusTimerId = window.setInterval(() => {
            void this.refreshLimitStatus(taskId);
        }, LIMIT_STATUS_POLL_INTERVAL_MS);
    }

    private stopLimitStatusPolling(): void {
        if (this.limitStatusTimerId !== null) {
            window.clearInterval(this.limitStatusTimerId);
            this.limitStatusTimerId = null;
        }
        this.stopLimitStatusBootstrapPolling();
    }

    private startLimitStatusBootstrapPolling(taskId: string): void {
        this.stopLimitStatusBootstrapPolling();
        this.limitStatusBootstrapAttempts = 0;
        this.limitStatusBootstrapTimerId = window.setInterval(() => {
            if (this.taskId !== taskId) {
                this.stopLimitStatusBootstrapPolling();
                return;
            }
            if (this.limitStatus !== null) {
                this.stopLimitStatusBootstrapPolling();
                return;
            }
            this.limitStatusBootstrapAttempts += 1;
            void this.refreshLimitStatus(taskId).finally(() => {
                if (this.taskId !== taskId) {
                    this.stopLimitStatusBootstrapPolling();
                    return;
                }
                if (
                    this.limitStatus !== null ||
                    this.limitStatusBootstrapAttempts >= LIMIT_STATUS_BOOTSTRAP_MAX_ATTEMPTS
                ) {
                    this.stopLimitStatusBootstrapPolling();
                }
            });
        }, LIMIT_STATUS_BOOTSTRAP_POLL_INTERVAL_MS);
    }

    private stopLimitStatusBootstrapPolling(): void {
        if (this.limitStatusBootstrapTimerId !== null) {
            window.clearInterval(this.limitStatusBootstrapTimerId);
            this.limitStatusBootstrapTimerId = null;
        }
        this.limitStatusBootstrapAttempts = 0;
    }

    private async refreshLimitStatus(taskId: string): Promise<void> {
        try {
            const limitStatus = await this.codexGuiStore.getLimitStatus(taskId);
            if (this.taskId !== taskId) {
                return;
            }
            this.zone.run(() => {
                this.limitStatus = limitStatus;
                this.cdr.markForCheck();
            });
        } catch {
            // Preserve the last successful snapshot during transient backend errors.
        }
    }

    private refreshEffortOptions(): void {
        const taskId = this.taskId;
        if (!taskId) {
            this.effortOptions = [];
            this.selectedEffort = "";
            return;
        }
        const efforts = this.codexGuiStore.getModelEfforts(
            taskId,
            this.selectedModel,
        );
        this.effortOptions = efforts.map((effort) => ({
            value: effort,
            label: effort,
        }));
        if (this.effortOptions.length === 0) {
            this.selectedEffort = "";
            this.codexGuiStore.setEffort(taskId, "");
            return;
        }
        if (!efforts.includes(this.selectedEffort)) {
            this.selectedEffort = efforts[0];
            this.codexGuiStore.setEffort(taskId, this.selectedEffort);
        }
    }
}
