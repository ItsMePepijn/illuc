import { Injectable, NgZone, OnDestroy } from "@angular/core";
import { type UnlistenFn } from "@tauri-apps/api/event";
import { Observable, Subject } from "rxjs";
import {
    ActivityEvent,
    HistoryEvent,
    HydratedEvent,
    LimitStatus,
    MessagePresentation,
    Message,
    MessageEvent,
    PlanEvent,
    RequestEvent,
    TokenUsageEvent,
    UsageSnapshot,
    WorkingPeriod,
} from "./models";
import { tauriInvoke, tauriListen } from "../../../shared/tauri/tauri-zone";
import { readIllucHmrState } from "../../../shared/hmr/hmr-state";

export type AgentChatActivityState = {
    label: string;
    startedAt: string | null;
};

export type AgentChatPlanState = {
    explanation: string | null;
    steps: Array<{
        step: string;
        status: string;
    }>;
};

export type AgentChatTokenUsageState = {
    totalTokens: number;
    inputTokens: number;
    cachedInputTokens: number;
    outputTokens: number;
    reasoningOutputTokens: number;
    lastTotalTokens: number;
    lastInputTokens: number;
    lastCachedInputTokens: number;
    lastOutputTokens: number;
    lastReasoningOutputTokens: number;
    modelContextWindow: number | null;
};

export type AgentChatRequestState = RequestEvent;

export type AgentChatCapabilitiesState = {
    supportsNewChat: boolean;
    supportsThreadHistory: boolean;
    supportsUsage: boolean;
    supportsServiceTierToggle: boolean;
};

@Injectable({
    providedIn: "root",
})
export class AgentChatStore implements OnDestroy {
    private readonly agentChatMessages = new Map<string, Message[]>();
    private readonly agentChatMessageStreams = new Map<
        string,
        Subject<Message[]>
    >();
    private readonly agentChatHydrated = new Map<string, boolean>();
    private readonly agentChatHydratedStreams = new Map<
        string,
        Subject<boolean>
    >();
    private readonly agentChatActivity = new Map<string, AgentChatActivityState>();
    private readonly agentChatActivityStreams = new Map<
        string,
        Subject<AgentChatActivityState | null>
    >();
    private readonly agentChatPlan = new Map<string, AgentChatPlanState | null>();
    private readonly agentChatPlanStreams = new Map<
        string,
        Subject<AgentChatPlanState | null>
    >();
    private readonly agentChatTokenUsage = new Map<
        string,
        AgentChatTokenUsageState | null
    >();
    private readonly agentChatTokenUsageStreams = new Map<
        string,
        Subject<AgentChatTokenUsageState | null>
    >();
    private readonly agentChatRequest = new Map<string, AgentChatRequestState | null>();
    private readonly agentChatRequestStreams = new Map<
        string,
        Subject<AgentChatRequestState | null>
    >();
    private readonly agentChatSelectedModel = new Map<string, string>();
    private readonly agentChatSelectedEffort = new Map<string, string>();
    private readonly agentChatSelectedServiceTier = new Map<string, string>();
    private readonly agentChatModelCapabilities = new Map<
        string,
        AgentChatModelCapability[]
    >();
    private readonly agentChatCapabilities = new Map<
        string,
        AgentChatCapabilitiesState
    >();
    private readonly agentChatMetadataRequests = new Map<
        string,
        Promise<AgentChatModelsResponse>
    >();
    private readonly unlistenFns: UnlistenFn[] = [];
    private readonly unloadHandler = () => this.teardown();

    constructor(private readonly zone: NgZone) {
        const snapshot = readIllucHmrState()?.agentChatStore;
        if (snapshot) {
            this.restoreDevState(snapshot);
        }
        this.registerEventListeners();
        window.addEventListener("unload", this.unloadHandler);
    }

    ngOnDestroy(): void {
        window.removeEventListener("unload", this.unloadHandler);
        this.teardown();
    }

    snapshotDevState(): AgentChatStoreDevState {
        return {
            messages: Object.fromEntries(this.agentChatMessages),
            hydrated: Object.fromEntries(this.agentChatHydrated),
            activity: Object.fromEntries(this.agentChatActivity),
            plan: Object.fromEntries(this.agentChatPlan),
            tokenUsage: Object.fromEntries(this.agentChatTokenUsage),
            request: Object.fromEntries(this.agentChatRequest),
            selectedModel: Object.fromEntries(this.agentChatSelectedModel),
            selectedEffort: Object.fromEntries(this.agentChatSelectedEffort),
            selectedServiceTier: Object.fromEntries(this.agentChatSelectedServiceTier),
            modelCapabilities: Object.fromEntries(this.agentChatModelCapabilities),
            capabilities: Object.fromEntries(this.agentChatCapabilities),
        };
    }

    restoreDevState(snapshot: AgentChatStoreDevState): void {
        this.restoreMap(this.agentChatMessages, snapshot.messages);
        this.restoreMap(this.agentChatHydrated, snapshot.hydrated);
        this.restoreMap(this.agentChatActivity, snapshot.activity);
        this.restoreMap(this.agentChatPlan, snapshot.plan);
        this.restoreMap(this.agentChatTokenUsage, snapshot.tokenUsage);
        this.restoreMap(this.agentChatRequest, snapshot.request);
        this.restoreMap(this.agentChatSelectedModel, snapshot.selectedModel);
        this.restoreMap(this.agentChatSelectedEffort, snapshot.selectedEffort);
        this.restoreMap(
            this.agentChatSelectedServiceTier,
            snapshot.selectedServiceTier,
        );
        this.restoreMap(
            this.agentChatModelCapabilities,
            snapshot.modelCapabilities,
        );
        this.restoreMap(this.agentChatCapabilities, snapshot.capabilities);
    }

    clearAll(): void {
        this.agentChatMessages.clear();
        this.agentChatMessageStreams.clear();
        this.agentChatHydrated.clear();
        this.agentChatHydratedStreams.clear();
        this.agentChatActivity.clear();
        this.agentChatActivityStreams.clear();
        this.agentChatPlan.clear();
        this.agentChatPlanStreams.clear();
        this.agentChatTokenUsage.clear();
        this.agentChatTokenUsageStreams.clear();
        this.agentChatRequest.clear();
        this.agentChatRequestStreams.clear();
        this.agentChatSelectedModel.clear();
        this.agentChatSelectedEffort.clear();
        this.agentChatSelectedServiceTier.clear();
        this.agentChatModelCapabilities.clear();
        this.agentChatCapabilities.clear();
        this.agentChatMetadataRequests.clear();
    }

    resetTask(taskId: string): void {
        if (!taskId) {
            return;
        }
        this.replaceAgentChatMessages(taskId, []);
        this.setAgentChatHydrated(taskId, false);
        this.setAgentChatActivity(taskId, null);
        this.setAgentChatPlan(taskId, null);
        this.setAgentChatTokenUsage(taskId, null);
        this.setAgentChatRequest(taskId, null);
        this.agentChatCapabilities.delete(taskId);
        this.agentChatMetadataRequests.delete(taskId);
    }

    removeTask(taskId: string): void {
        this.agentChatMessages.delete(taskId);
        this.agentChatMessageStreams.delete(taskId);
        this.agentChatHydrated.delete(taskId);
        this.agentChatHydratedStreams.delete(taskId);
        this.agentChatActivity.delete(taskId);
        this.agentChatActivityStreams.delete(taskId);
        this.agentChatPlan.delete(taskId);
        this.agentChatPlanStreams.delete(taskId);
        this.agentChatTokenUsage.delete(taskId);
        this.agentChatTokenUsageStreams.delete(taskId);
        this.agentChatRequest.delete(taskId);
        this.agentChatRequestStreams.delete(taskId);
        this.agentChatSelectedModel.delete(taskId);
        this.agentChatSelectedEffort.delete(taskId);
        this.agentChatSelectedServiceTier.delete(taskId);
        this.agentChatModelCapabilities.delete(taskId);
        this.agentChatCapabilities.delete(taskId);
        this.agentChatMetadataRequests.delete(taskId);
    }

    async sendMessage(
        taskId: string,
        content: string,
        model?: string | null,
        effort?: string | null,
        serviceTier?: string | null,
    ): Promise<void> {
        const text = content.trim();
        if (!text) {
            return;
        }
        const resolvedModel = (model ?? this.getModel(taskId)).trim();
        const resolvedEffort = (effort ?? this.getEffort(taskId)).trim();
        const resolvedServiceTier = (
            serviceTier ?? this.getServiceTier(taskId)
        ).trim();
        this.setTrimmedValue(this.agentChatSelectedModel, taskId, resolvedModel);
        this.setTrimmedValue(this.agentChatSelectedEffort, taskId, resolvedEffort);
        this.setTrimmedValue(
            this.agentChatSelectedServiceTier,
            taskId,
            resolvedServiceTier,
        );
        this.pushAgentChatMessage(taskId, {
            id: this.newMessageId(),
            role: "user",
            content: text,
            presentation: {
                kind: "user",
                text,
                textFormat: "markdown",
                toolRows: [],
                toolStatusLabel: null,
                isToolRunning: false,
            },
            createdAt: new Date().toISOString(),
            status: "complete",
        });
        try {
            await tauriInvoke<void>(this.zone, "task_agent_gui_send", {
                req: {
                    taskId,
                    content: text,
                    model: resolvedModel || undefined,
                    effort: resolvedEffort || undefined,
                    serviceTier: resolvedServiceTier || undefined,
                },
            });
        } catch (error) {
            const detail = error instanceof Error ? error.message : String(error);
            this.pushAgentChatMessage(taskId, {
                id: this.newMessageId(),
                role: "system",
                content: `GUI agent send failed: ${detail}`,
                presentation: {
                    kind: "standard",
                    text: `GUI agent send failed: ${detail}`,
                    textFormat: "markdown",
                    toolRows: [],
                    toolStatusLabel: null,
                    isToolRunning: false,
                },
                createdAt: new Date().toISOString(),
                status: "error",
            });
            throw error;
        }
    }

    interruptTask(taskId: string): Promise<void> {
        return tauriInvoke<void>(this.zone, "task_agent_gui_interrupt", {
            req: { taskId },
        });
    }

    respondToRequest(
        taskId: string,
        requestId: string,
        response: unknown,
    ): Promise<void> {
        return tauriInvoke<void>(this.zone, "task_agent_gui_request_respond", {
            req: { taskId, requestId, response },
        });
    }

    compactThread(taskId: string): Promise<void> {
        return tauriInvoke<void>(this.zone, "task_agent_gui_compact", {
            req: { taskId },
        });
    }

    async newChat(taskId: string): Promise<void> {
        await tauriInvoke<void>(this.zone, "task_agent_gui_new_chat", {
            req: { taskId },
        });
        this.replaceAgentChatMessages(taskId, []);
        this.setAgentChatHydrated(taskId, true);
        this.setAgentChatActivity(taskId, null);
        this.setAgentChatPlan(taskId, null);
        this.setAgentChatTokenUsage(taskId, null);
        this.setAgentChatRequest(taskId, null);
    }

    async rollbackThread(taskId: string, numTurns = 1): Promise<void> {
        const response = await tauriInvoke<AgentChatRollbackResponse>(
            this.zone,
            "task_agent_gui_rollback",
            { req: { taskId, numTurns } },
        );
        this.setAgentChatPlan(taskId, null);
        this.replaceAgentChatMessages(
            taskId,
            (response.events ?? []).map((event) => ({
                id: event.messageId,
                role: event.role as Message["role"],
                content: event.content,
                presentation: event.presentation,
                createdAt: new Date().toISOString(),
                status: event.isFinal ? "complete" : "streaming",
            })),
        );
        this.setAgentChatHydrated(taskId, true);
    }

    messages$(taskId: string): Observable<Message[]> {
        return this.ensureAgentChatMessageStream(taskId).asObservable();
    }

    hydrated$(taskId: string): Observable<boolean> {
        return this.ensureAgentChatHydratedStream(taskId).asObservable();
    }

    activity$(taskId: string): Observable<AgentChatActivityState | null> {
        return this.ensureAgentChatActivityStream(taskId).asObservable();
    }

    plan$(taskId: string): Observable<AgentChatPlanState | null> {
        return this.ensureAgentChatPlanStream(taskId).asObservable();
    }

    tokenUsage$(taskId: string): Observable<AgentChatTokenUsageState | null> {
        return this.ensureAgentChatTokenUsageStream(taskId).asObservable();
    }

    request$(taskId: string): Observable<AgentChatRequestState | null> {
        return this.ensureAgentChatRequestStream(taskId).asObservable();
    }

    getMessages(taskId: string): Message[] {
        return this.agentChatMessages.get(taskId) ?? [];
    }

    isHydrated(taskId: string): boolean {
        return this.agentChatHydrated.get(taskId) ?? false;
    }

    getActivity(taskId: string): AgentChatActivityState | null {
        return this.agentChatActivity.get(taskId) ?? null;
    }

    getPlan(taskId: string): AgentChatPlanState | null {
        return this.agentChatPlan.get(taskId) ?? null;
    }

    getTokenUsage(taskId: string): AgentChatTokenUsageState | null {
        return this.agentChatTokenUsage.get(taskId) ?? null;
    }

    getRequest(taskId: string): AgentChatRequestState | null {
        return this.agentChatRequest.get(taskId) ?? null;
    }

    getModel(taskId: string): string {
        return this.agentChatSelectedModel.get(taskId) ?? "";
    }

    setModel(taskId: string, model: string): void {
        this.setTrimmedValue(this.agentChatSelectedModel, taskId, model);
    }

    async getAvailableModels(taskId: string): Promise<string[]> {
        if (!taskId) {
            return [];
        }
        const response = await this.fetchAgentChatMetadata(taskId);
        return response.models ?? [];
    }

    getEffort(taskId: string): string {
        return this.agentChatSelectedEffort.get(taskId) ?? "";
    }

    setEffort(taskId: string, effort: string): void {
        this.setTrimmedValue(this.agentChatSelectedEffort, taskId, effort);
    }

    getServiceTier(taskId: string): string {
        return this.agentChatSelectedServiceTier.get(taskId) ?? "flex";
    }

    getCapabilities(taskId: string): AgentChatCapabilitiesState {
        return (
            this.agentChatCapabilities.get(taskId) ?? defaultAgentChatCapabilities()
        );
    }

    setServiceTier(taskId: string, serviceTier: string): void {
        this.setTrimmedValue(
            this.agentChatSelectedServiceTier,
            taskId,
            serviceTier,
        );
    }

    async ensureCapabilities(taskId: string): Promise<AgentChatCapabilitiesState> {
        if (!taskId) {
            return defaultAgentChatCapabilities();
        }
        const response = await this.fetchAgentChatMetadata(taskId);
        return normalizeAgentChatCapabilities(response.capabilities);
    }

    getModelEfforts(taskId: string, model: string): string[] {
        const capabilities = this.agentChatModelCapabilities.get(taskId) ?? [];
        const selected = capabilities.find((item) => item.model === model);
        return selected?.reasoningEfforts ?? [];
    }

    async getUsage(taskId: string): Promise<UsageSnapshot | null> {
        if (!taskId) {
            return null;
        }
        const response = await tauriInvoke<AgentChatUsageResponse>(
            this.zone,
            "task_agent_gui_usage",
            { req: { taskId } },
        );
        return this.parseAgentChatUsage(response);
    }

    async getLimitStatus(taskId: string): Promise<LimitStatus | null> {
        if (!taskId) {
            return null;
        }
        const response = await tauriInvoke<AgentChatUsageResponse>(
            this.zone,
            "task_agent_gui_usage",
            { req: { taskId } },
        );
        return this.parseAgentChatLimitStatus(response.rateLimits);
    }

    private async fetchAgentChatMetadata(
        taskId: string,
    ): Promise<AgentChatModelsResponse> {
        const pending = this.agentChatMetadataRequests.get(taskId);
        if (pending) {
            return pending;
        }
        const request = tauriInvoke<AgentChatModelsResponse>(
            this.zone,
            "task_agent_gui_models",
            { req: { taskId } },
        )
            .then((response) => {
                this.agentChatModelCapabilities.set(
                    taskId,
                    response.modelCapabilities ?? [],
                );
                this.setTrimmedValue(
                    this.agentChatSelectedModel,
                    taskId,
                    response.selectedModel ?? "",
                );
                this.setTrimmedValue(
                    this.agentChatSelectedEffort,
                    taskId,
                    response.selectedEffort ?? "",
                );
                this.setTrimmedValue(
                    this.agentChatSelectedServiceTier,
                    taskId,
                    response.selectedServiceTier ?? "",
                );
                this.agentChatCapabilities.set(
                    taskId,
                    normalizeAgentChatCapabilities(response.capabilities),
                );
                return response;
            })
            .finally(() => {
                this.agentChatMetadataRequests.delete(taskId);
            });
        this.agentChatMetadataRequests.set(taskId, request);
        return request;
    }

    private registerEventListeners(): void {
        void tauriListen<MessageEvent>(
            this.zone,
            "task_agent_gui_message",
            (event) => {
                this.applyAgentChatEvent(event.payload);
            },
        ).then((unlisten) => this.unlistenFns.push(unlisten));

        void tauriListen<HistoryEvent>(
            this.zone,
            "task_agent_gui_history",
            (event) => {
                this.applyAgentChatHistory(event.payload);
            },
        ).then((unlisten) => this.unlistenFns.push(unlisten));

        void tauriListen<HydratedEvent>(
            this.zone,
            "task_agent_gui_hydrated",
            (event) => {
                this.setAgentChatHydrated(event.payload.taskId, true);
            },
        ).then((unlisten) => this.unlistenFns.push(unlisten));

        void tauriListen<ActivityEvent>(
            this.zone,
            "task_agent_gui_activity",
            (event) => {
                this.applyAgentChatActivity(event.payload);
            },
        ).then((unlisten) => this.unlistenFns.push(unlisten));

        void tauriListen<PlanEvent>(
            this.zone,
            "task_agent_gui_plan",
            (event) => {
                this.applyAgentChatPlan(event.payload);
            },
        ).then((unlisten) => this.unlistenFns.push(unlisten));

        void tauriListen<TokenUsageEvent>(
            this.zone,
            "task_agent_gui_token_usage",
            (event) => {
                this.applyAgentChatTokenUsage(event.payload);
            },
        ).then((unlisten) => this.unlistenFns.push(unlisten));

        void tauriListen<RequestEvent>(
            this.zone,
            "task_agent_gui_request",
            (event) => {
                this.applyAgentChatRequest(event.payload);
            },
        ).then((unlisten) => this.unlistenFns.push(unlisten));
    }

    private ensureAgentChatMessageStream(
        taskId: string,
    ): Subject<Message[]> {
        let stream = this.agentChatMessageStreams.get(taskId);
        if (!stream) {
            stream = new Subject<Message[]>();
            this.agentChatMessageStreams.set(taskId, stream);
        }
        return stream;
    }

    private ensureAgentChatHydratedStream(taskId: string): Subject<boolean> {
        let stream = this.agentChatHydratedStreams.get(taskId);
        if (!stream) {
            stream = new Subject<boolean>();
            this.agentChatHydratedStreams.set(taskId, stream);
        }
        return stream;
    }

    private ensureAgentChatActivityStream(
        taskId: string,
    ): Subject<AgentChatActivityState | null> {
        let stream = this.agentChatActivityStreams.get(taskId);
        if (!stream) {
            stream = new Subject<AgentChatActivityState | null>();
            this.agentChatActivityStreams.set(taskId, stream);
        }
        return stream;
    }

    private ensureAgentChatPlanStream(
        taskId: string,
    ): Subject<AgentChatPlanState | null> {
        let stream = this.agentChatPlanStreams.get(taskId);
        if (!stream) {
            stream = new Subject<AgentChatPlanState | null>();
            this.agentChatPlanStreams.set(taskId, stream);
        }
        return stream;
    }

    private ensureAgentChatTokenUsageStream(
        taskId: string,
    ): Subject<AgentChatTokenUsageState | null> {
        let stream = this.agentChatTokenUsageStreams.get(taskId);
        if (!stream) {
            stream = new Subject<AgentChatTokenUsageState | null>();
            this.agentChatTokenUsageStreams.set(taskId, stream);
        }
        return stream;
    }

    private ensureAgentChatRequestStream(
        taskId: string,
    ): Subject<AgentChatRequestState | null> {
        let stream = this.agentChatRequestStreams.get(taskId);
        if (!stream) {
            stream = new Subject<AgentChatRequestState | null>();
            this.agentChatRequestStreams.set(taskId, stream);
        }
        return stream;
    }

    private setAgentChatHydrated(taskId: string, hydrated: boolean): void {
        if (!taskId) {
            return;
        }
        this.agentChatHydrated.set(taskId, hydrated);
        this.ensureAgentChatHydratedStream(taskId).next(hydrated);
    }

    private setAgentChatPlan(taskId: string, plan: AgentChatPlanState | null): void {
        if (!taskId) {
            return;
        }
        if (plan) {
            this.agentChatPlan.set(taskId, plan);
        } else {
            this.agentChatPlan.delete(taskId);
        }
        this.ensureAgentChatPlanStream(taskId).next(plan);
    }

    private setAgentChatTokenUsage(
        taskId: string,
        usage: AgentChatTokenUsageState | null,
    ): void {
        if (!taskId) {
            return;
        }
        if (usage) {
            this.agentChatTokenUsage.set(taskId, usage);
        } else {
            this.agentChatTokenUsage.delete(taskId);
        }
        this.ensureAgentChatTokenUsageStream(taskId).next(usage);
    }

    private setAgentChatRequest(
        taskId: string,
        request: AgentChatRequestState | null,
    ): void {
        if (!taskId) {
            return;
        }
        if (request) {
            this.agentChatRequest.set(taskId, request);
        } else {
            this.agentChatRequest.delete(taskId);
        }
        this.ensureAgentChatRequestStream(taskId).next(request);
    }

    private applyAgentChatActivity(event: ActivityEvent): void {
        if (!event.taskId) {
            return;
        }
        const label = event.label?.trim() ?? "";
        const startedAt = event.startedAt?.trim() ?? "";
        const activity =
            label.length > 0
                ? {
                      label,
                      startedAt: startedAt.length > 0 ? startedAt : null,
                  }
                : null;
        this.setAgentChatActivity(event.taskId, activity);
    }

    private applyAgentChatPlan(event: PlanEvent): void {
        const explanation = event.explanation?.trim() || null;
        const steps = (event.plan ?? []).filter((item) => item.step?.trim().length > 0);
        this.setAgentChatPlan(
            event.taskId,
            explanation || steps.length > 0
                ? {
                      explanation,
                      steps: steps.map((item) => ({
                          step: item.step,
                          status: item.status,
                      })),
                  }
                : null,
        );
    }

    private applyAgentChatTokenUsage(event: TokenUsageEvent): void {
        this.setAgentChatTokenUsage(event.taskId, event.usage ?? null);
    }

    private applyAgentChatRequest(event: RequestEvent): void {
        const kind = event.kind?.trim() ?? "";
        this.setAgentChatRequest(
            event.taskId,
            kind && kind !== "none" && event.requestId
                ? {
                      ...event,
                      kind,
                  }
                : null,
        );
    }

    private setAgentChatActivity(
        taskId: string,
        activity: AgentChatActivityState | null,
    ): void {
        if (!taskId) {
            return;
        }
        if (activity) {
            this.agentChatActivity.set(taskId, activity);
        } else {
            this.agentChatActivity.delete(taskId);
        }
        this.ensureAgentChatActivityStream(taskId).next(activity);
    }

    private pushAgentChatMessage(taskId: string, message: Message): void {
        if (!taskId) {
            return;
        }
        const current = this.agentChatMessages.get(taskId) ?? [];
        const updated = [...current, message];
        this.agentChatMessages.set(taskId, updated);
        this.ensureAgentChatMessageStream(taskId).next(updated);
    }

    private replaceAgentChatMessages(taskId: string, messages: Message[]): void {
        this.agentChatMessages.set(taskId, messages);
        this.ensureAgentChatMessageStream(taskId).next(messages);
    }

    private applyAgentChatHistory(event: HistoryEvent): void {
        if (!event.taskId) {
            return;
        }
        this.replaceAgentChatMessages(
            event.taskId,
            (event.events ?? []).map((item) => ({
                id: item.messageId,
                role: item.role,
                content: item.content,
                presentation: item.presentation,
                createdAt: new Date().toISOString(),
                status: item.isFinal ? "complete" : "streaming",
            })),
        );
    }

    private applyAgentChatEvent(event: MessageEvent): void {
        const current = this.agentChatMessages.get(event.taskId) ?? [];
        const existingIndex = current.findIndex((message) => message.id === event.messageId);
        const incomingContent = event.content ?? "";
        const hasIncomingContent = incomingContent.length > 0;
        if (existingIndex >= 0) {
            const existing = current[existingIndex];
            const mergedContent = event.isDelta
                ? `${existing.content}${incomingContent}`
                : incomingContent || existing.content;
            const updatedMessage: Message = {
                ...existing,
                content: mergedContent,
                role: event.role,
                presentation: this.mergeAgentChatPresentation(
                    existing.presentation,
                    event.presentation,
                    mergedContent,
                    event.isDelta,
                ),
                status: event.isFinal ? "complete" : existing.status,
            };
            const updated = [...current];
            updated[existingIndex] = updatedMessage;
            this.agentChatMessages.set(event.taskId, updated);
            this.ensureAgentChatMessageStream(event.taskId).next(updated);
            return;
        }

        if (!hasIncomingContent) {
            return;
        }

        this.pushAgentChatMessage(event.taskId, {
            id: event.messageId,
            role: event.role,
            content: incomingContent,
            presentation: event.presentation,
            createdAt: new Date().toISOString(),
            status: event.isFinal ? "complete" : "streaming",
        });
    }

    private mergeAgentChatPresentation(
        existing: MessagePresentation,
        incoming: MessagePresentation,
        mergedContent: string,
        isDelta: boolean,
    ): MessagePresentation {
        if (!isDelta) {
            return incoming;
        }

        if (incoming.kind === "tool") {
            return incoming;
        }

        return {
            ...existing,
            ...incoming,
            text: mergedContent,
        };
    }

    private parseAgentChatUsage(response: AgentChatUsageResponse): UsageSnapshot | null {
        const envelope = this.agentChatRateLimitEnvelope(response.rateLimits);
        if (!envelope) {
            return null;
        }

        const primary = envelope["primary"] as Record<string, unknown> | undefined;
        const secondary = envelope["secondary"] as Record<string, unknown> | undefined;
        const weekly =
            secondary &&
            Number(secondary["windowDurationMins"]) >= Number(primary?.["windowDurationMins"] ?? 0)
                ? secondary
                : primary ?? secondary;
        return this.parseUsageSnapshot(
            weekly,
            response.windowDurationHours,
            response.workingPeriods,
        );
    }

    private parseAgentChatLimitStatus(rateLimits: unknown): LimitStatus | null {
        const envelope = this.agentChatRateLimitEnvelope(rateLimits);
        if (!envelope) {
            return null;
        }

        const primary = envelope["primary"] as Record<string, unknown> | undefined;
        if (!primary) {
            return null;
        }

        const usedPercent = Number(primary["usedPercent"]);
        const windowDurationMins = Number(primary["windowDurationMins"]);
        if (!Number.isFinite(usedPercent) || !Number.isFinite(windowDurationMins)) {
            return null;
        }

        return {
            windowDurationMins,
            usedPercent: Math.min(100, Math.max(0, usedPercent)),
        };
    }

    private agentChatRateLimitEnvelope(
        rateLimits: unknown,
    ): Record<string, unknown> | null {
        const root =
            rateLimits && typeof rateLimits === "object"
                ? (rateLimits as Record<string, unknown>)
                : null;
        if (!root) {
            return null;
        }

        const envelope =
            (root["rateLimits"] as Record<string, unknown> | undefined) ?? root;
        if (!envelope || typeof envelope !== "object") {
            return null;
        }
        return envelope;
    }

    private parseUsageSnapshot(
        window: Record<string, unknown> | undefined,
        windowDurationHours?: number | null,
        workingPeriods?: WorkingPeriod[],
    ): UsageSnapshot | null {
        if (!window) {
            return null;
        }

        const usedPercent = Number(window["usedPercent"]);
        const resetsAt = Number(window["resetsAt"]);
        if (!Number.isFinite(usedPercent) || !Number.isFinite(resetsAt)) {
            return null;
        }

        const resetAt = new Date(resetsAt * 1000);
        if (Number.isNaN(resetAt.getTime())) {
            return null;
        }

        const used = Math.min(100, Math.max(0, usedPercent));
        return {
            used,
            limit: 100,
            resetAt: resetAt.toISOString(),
            windowDurationHours:
                typeof windowDurationHours === "number" &&
                Number.isFinite(windowDurationHours) &&
                windowDurationHours > 0
                    ? windowDurationHours
                    : null,
            workingPeriods: (workingPeriods ?? []).filter(
                (period) =>
                    typeof period?.startAt === "string" &&
                    period.startAt.length > 0 &&
                    typeof period?.endAt === "string" &&
                    period.endAt.length > 0,
            ),
        };
    }

    private newMessageId(): string {
        if (typeof crypto !== "undefined" && "randomUUID" in crypto) {
            return crypto.randomUUID();
        }
        return `msg-${Date.now()}-${Math.random().toString(16).slice(2)}`;
    }

    private teardown(): void {
        while (this.unlistenFns.length > 0) {
            const unlisten = this.unlistenFns.pop();
            if (unlisten) {
                void unlisten();
            }
        }
    }

    private setTrimmedValue(
        map: Map<string, string>,
        taskId: string,
        value: string | null | undefined,
    ): void {
        if (!taskId) {
            return;
        }
        const normalized = (value ?? "").trim();
        if (!normalized) {
            map.delete(taskId);
            return;
        }
        map.set(taskId, normalized);
    }

    private restoreMap<T>(
        target: Map<string, T>,
        source: Record<string, T> | undefined,
    ): void {
        target.clear();
        for (const [key, value] of Object.entries(source ?? {})) {
            target.set(key, value);
        }
    }
}

export type AgentChatStoreDevState = {
    messages: Record<string, Message[]>;
    hydrated: Record<string, boolean>;
    activity: Record<string, AgentChatActivityState>;
    plan: Record<string, AgentChatPlanState | null>;
    tokenUsage: Record<string, AgentChatTokenUsageState | null>;
    request: Record<string, AgentChatRequestState | null>;
    selectedModel: Record<string, string>;
    selectedEffort: Record<string, string>;
    selectedServiceTier: Record<string, string>;
    modelCapabilities: Record<string, AgentChatModelCapability[]>;
    capabilities: Record<string, AgentChatCapabilitiesState>;
};

type AgentChatModelsResponse = {
    models: string[];
    modelCapabilities: AgentChatModelCapability[];
    selectedModel?: string | null;
    selectedEffort?: string | null;
    selectedServiceTier?: string | null;
    capabilities?: Partial<AgentChatCapabilitiesState> | null;
};

type AgentChatUsageResponse = {
    rateLimits?: unknown;
    windowDurationHours?: number | null;
    workingPeriods?: WorkingPeriod[];
};

type AgentChatModelCapability = {
    model: string;
    reasoningEfforts: string[];
};

type AgentChatRollbackResponse = {
    events: Array<{
        messageId: string;
        role: string;
        content: string;
        presentation: MessagePresentation;
        isDelta: boolean;
        isFinal: boolean;
    }>;
};

function defaultAgentChatCapabilities(): AgentChatCapabilitiesState {
    return {
        supportsNewChat: false,
        supportsThreadHistory: false,
        supportsUsage: false,
        supportsServiceTierToggle: false,
    };
}

function normalizeAgentChatCapabilities(
    value: Partial<AgentChatCapabilitiesState> | null | undefined,
): AgentChatCapabilitiesState {
    return {
        supportsNewChat: value?.supportsNewChat === true,
        supportsThreadHistory: value?.supportsThreadHistory === true,
        supportsUsage: value?.supportsUsage === true,
        supportsServiceTierToggle: value?.supportsServiceTierToggle === true,
    };
}
