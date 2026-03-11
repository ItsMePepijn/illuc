import { CommonModule } from "@angular/common";
import {
    ChangeDetectionStrategy,
    ChangeDetectorRef,
    Component,
    Input,
    OnChanges,
    OnDestroy,
    SimpleChanges,
} from "@angular/core";
import { FormsModule } from "@angular/forms";
import { Subscription } from "rxjs";
import { LimitStatus, Message } from "../../models";
import {
    AgentChatActivityState,
    AgentChatSlashCommand,
    AgentChatStore,
    AgentChatPlanState,
    AgentChatRequestState,
    AgentChatTokenUsageState,
} from "../../agent-chat.store";
import { TaskStore } from "../../../task.store";
import {
    ComposerComponent,
    AgentChatModelOption,
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
    selector: "app-agent-chat",
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
export class AgentChatComponent implements OnChanges, OnDestroy {
    @Input() taskId: string | null = null;
    @Input() isWorking = false;
    @Input() stripPathPrefix = "";
    @Input() isActive = true;

    messages: Message[] = [];
    messagesHydrated = false;
    activity: AgentChatActivityState | null = null;
    plan: AgentChatPlanState | null = null;
    tokenUsage: AgentChatTokenUsageState | null = null;
    limitStatus: LimitStatus | null = null;
    pendingRequest: AgentChatRequestState | null = null;
    prompt = "";
    sending = false;
    selectedModel = "";
    selectedEffort = "";
    selectedServiceTier = "";
    modelOptions: AgentChatModelOption[] = [];
    effortOptions: AgentChatModelOption[] = [];
    slashCommands: AgentChatSlashCommand[] = [];
    showServiceTierToggle = false;
    showUsageRail = false;
    requestAnswers: Record<string, string[]> = {};

    readonly fetchUsageForRail = async (
        taskId: string,
    ): Promise<UsageWindowSnapshot | null> =>
        this.agentChatStore.getCapabilities(taskId).supportsUsage
            ? this.agentChatStore.getUsage(taskId)
            : null;

    private messageSubscription?: Subscription;
    private hydrationSubscription?: Subscription;
    private activitySubscription?: Subscription;
    private planSubscription?: Subscription;
    private tokenUsageSubscription?: Subscription;
    private requestSubscription?: Subscription;
    private limitStatusTimerId: number | null = null;
    private limitStatusBootstrapTimerId: number | null = null;
    private limitStatusBootstrapAttempts = 0;
    private metadataRefreshInFlight = false;

    constructor(
        private readonly taskStore: TaskStore,
        private readonly agentChatStore: AgentChatStore,
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
            await this.agentChatStore.sendMessage(
                this.taskId,
                text,
                this.selectedModel,
                this.selectedEffort,
                this.selectedServiceTier,
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
        this.agentChatStore.setModel(this.taskId, model);
        this.refreshEffortOptions();
        this.cdr.markForCheck();
    }

    onEffortChange(effort: string): void {
        if (!this.taskId) {
            return;
        }
        this.selectedEffort = effort;
        this.agentChatStore.setEffort(this.taskId, effort);
        this.cdr.markForCheck();
    }

    onServiceTierToggleRequested(): void {
        if (!this.taskId) {
            return;
        }
        const nextServiceTier =
            this.selectedServiceTier === "fast" ? "flex" : "fast";
        this.selectedServiceTier = nextServiceTier;
        this.agentChatStore.setServiceTier(this.taskId, nextServiceTier);
        this.cdr.markForCheck();
    }

    async stop(): Promise<void> {
        if (!this.taskId) {
            return;
        }
        try {
            await this.agentChatStore.interruptTask(this.taskId);
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
        await this.agentChatStore.respondToRequest(
            this.taskId,
            this.pendingRequest.requestId,
            { decision },
        );
        this.requestAnswers = {};
        this.cdr.markForCheck();
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
        await this.agentChatStore.respondToRequest(
            this.taskId,
            this.pendingRequest.requestId,
            { answers },
        );
        this.requestAnswers = {};
        this.cdr.markForCheck();
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
            this.selectedServiceTier = "";
            this.modelOptions = [];
            this.effortOptions = [];
            this.slashCommands = [];
            this.showServiceTierToggle = false;
            this.showUsageRail = false;
            this.cdr.markForCheck();
            return;
        }

        this.messages = this.agentChatStore.getMessages(this.taskId);
        this.messagesHydrated =
            this.messages.length > 0 ||
            this.agentChatStore.isHydrated(this.taskId);
        this.activity = this.agentChatStore.getActivity(this.taskId);
        this.plan = this.agentChatStore.getPlan(this.taskId);
        this.tokenUsage = this.agentChatStore.getTokenUsage(this.taskId);
        this.limitStatus = null;
        this.pendingRequest = this.agentChatStore.getRequest(this.taskId);
        this.selectedModel = this.agentChatStore.getModel(this.taskId);
        this.selectedEffort = this.agentChatStore.getEffort(this.taskId);
        this.selectedServiceTier = this.agentChatStore.getServiceTier(this.taskId);
        this.slashCommands = this.agentChatStore.getSlashCommands(this.taskId);
        this.applyCapabilities(this.taskId);
        this.messageSubscription = this.agentChatStore
            .messages$(this.taskId)
            .subscribe((messages) => {
                const shouldRefreshMetadata =
                    this.shouldRefreshMetadataFromMessages(
                        this.messages,
                        messages,
                    );
                this.messages = messages;
                if (messages.length > 0) {
                    this.messagesHydrated = true;
                }
                if (shouldRefreshMetadata) {
                    void this.loadAvailableModels();
                }
                this.cdr.markForCheck();
            });
        this.hydrationSubscription = this.agentChatStore
            .hydrated$(this.taskId)
            .subscribe((hydrated) => {
                this.messagesHydrated = hydrated || this.messages.length > 0;
                if (hydrated && this.shouldRefreshMetadata()) {
                    void this.loadAvailableModels();
                }
                this.cdr.markForCheck();
            });
        this.activitySubscription = this.agentChatStore
            .activity$(this.taskId)
            .subscribe((activity) => {
                this.activity = activity;
                this.cdr.markForCheck();
            });
        this.planSubscription = this.agentChatStore
            .plan$(this.taskId)
            .subscribe((plan) => {
                this.plan = plan;
                this.cdr.markForCheck();
            });
        this.tokenUsageSubscription = this.agentChatStore
            .tokenUsage$(this.taskId)
            .subscribe((tokenUsage) => {
                this.tokenUsage = tokenUsage;
                this.cdr.markForCheck();
            });
        this.requestSubscription = this.agentChatStore
            .request$(this.taskId)
            .subscribe((request) => {
                this.pendingRequest = request;
                this.requestAnswers = this.buildInitialRequestAnswers(request);
                this.cdr.markForCheck();
            });

        void this.loadAvailableModels();
        this.cdr.markForCheck();
    }

    private buildInitialRequestAnswers(
        request: AgentChatRequestState | null,
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
        if (this.metadataRefreshInFlight) {
            return;
        }
        this.metadataRefreshInFlight = true;
        try {
            let models: string[] = [];
            for (let attempt = 0; attempt < 10; attempt += 1) {
                models = await this.agentChatStore.getAvailableModels(taskId);
                const selectedEffort = this.agentChatStore.getEffort(taskId);
                const hasEfforts =
                    models.some(
                        (model) =>
                            this.agentChatStore.getModelEfforts(taskId, model).length > 0,
                    ) || selectedEffort.length > 0;
                if ((models.length > 0 && hasEfforts) || attempt === 9) {
                    break;
                }
                await new Promise((resolve) => window.setTimeout(resolve, 400));
            }
            if (this.taskId !== taskId) {
                return;
            }
            this.selectedModel = this.agentChatStore.getModel(taskId);
            this.selectedEffort = this.agentChatStore.getEffort(taskId);
            this.selectedServiceTier = this.agentChatStore.getServiceTier(taskId);
            this.slashCommands = this.agentChatStore.getSlashCommands(taskId);
            this.applyCapabilities(taskId);
            this.modelOptions = models.map((model) => ({
                value: model,
                label: model.replaceAll("-", " "),
            }));
            if (this.selectedModel && !models.includes(this.selectedModel)) {
                this.selectedModel = "";
                this.agentChatStore.setModel(taskId, "");
            }
            this.refreshEffortOptions();
            this.cdr.markForCheck();
        } catch {
            // keep fallback
        } finally {
            this.metadataRefreshInFlight = false;
        }
    }

    private shouldRefreshMetadata(): boolean {
        return this.modelOptions.length === 0 || this.effortOptions.length === 0;
    }

    private shouldRefreshMetadataFromMessages(
        previousMessages: Message[],
        nextMessages: Message[],
    ): boolean {
        if (!this.shouldRefreshMetadata()) {
            return false;
        }
        if (nextMessages.length <= previousMessages.length) {
            return false;
        }

        return nextMessages
            .slice(previousMessages.length)
            .some(
                (message) =>
                    message.role === "system" || message.presentation.kind === "tool",
            );
    }

    private startLimitStatusPolling(taskId: string): void {
        if (!this.showUsageRail) {
            this.stopLimitStatusPolling();
            this.limitStatus = null;
            return;
        }
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
        if (!this.showUsageRail) {
            this.limitStatus = null;
            return;
        }
        try {
            const limitStatus = await this.agentChatStore.getLimitStatus(taskId);
            if (this.taskId !== taskId) {
                return;
            }
            this.limitStatus = limitStatus;
            this.cdr.markForCheck();
        } catch {
            // Preserve the last successful snapshot during transient backend errors.
        }
    }

    private applyCapabilities(taskId: string): void {
        const capabilities = this.agentChatStore.getCapabilities(taskId);
        this.showServiceTierToggle = capabilities.supportsServiceTierToggle;
        this.showUsageRail = capabilities.supportsUsage;
        if (this.showUsageRail) {
            this.startLimitStatusPolling(taskId);
        } else {
            this.stopLimitStatusPolling();
            this.limitStatus = null;
        }
    }


    private refreshEffortOptions(): void {
        const taskId = this.taskId;
        if (!taskId) {
            this.effortOptions = [];
            this.selectedEffort = "";
            return;
        }
        const efforts = this.agentChatStore.getModelEfforts(
            taskId,
            this.selectedModel,
        );
        this.effortOptions = efforts.map((effort) => ({
            value: effort,
            label: effort,
        }));
        if (this.effortOptions.length === 0) {
            this.selectedEffort = "";
            this.agentChatStore.setEffort(taskId, "");
            return;
        }
        if (!efforts.includes(this.selectedEffort)) {
            this.selectedEffort = efforts[0];
            this.agentChatStore.setEffort(taskId, this.selectedEffort);
        }
    }
}
