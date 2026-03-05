import { Injectable, NgZone } from "@angular/core";
import { type UnlistenFn } from "@tauri-apps/api/event";
import { Observable, Subject } from "rxjs";
import {
    ActivityEvent,
    HydratedEvent,
    LimitStatus,
    Message,
    MessageEvent,
    PlanEvent,
    RequestEvent,
    TokenUsageEvent,
    UsageSnapshot,
    WorkingPeriod,
} from "./models";
import { tauriInvoke, tauriListen } from "../../../shared/tauri/tauri-zone";

export type CodexGuiActivityState = {
    label: string;
    startedAt: string | null;
};

export type CodexGuiPlanState = {
    explanation: string | null;
    steps: Array<{
        step: string;
        status: string;
    }>;
};

export type CodexGuiTokenUsageState = {
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

export type CodexGuiRequestState = RequestEvent;

@Injectable({
    providedIn: "root",
})
export class CodexGuiStore {
    private readonly codexGuiMessages = new Map<string, Message[]>();
    private readonly codexGuiMessageStreams = new Map<
        string,
        Subject<Message[]>
    >();
    private readonly codexGuiHydrated = new Map<string, boolean>();
    private readonly codexGuiHydratedStreams = new Map<
        string,
        Subject<boolean>
    >();
    private readonly codexGuiActivity = new Map<string, CodexGuiActivityState>();
    private readonly codexGuiActivityStreams = new Map<
        string,
        Subject<CodexGuiActivityState | null>
    >();
    private readonly codexGuiPlan = new Map<string, CodexGuiPlanState | null>();
    private readonly codexGuiPlanStreams = new Map<
        string,
        Subject<CodexGuiPlanState | null>
    >();
    private readonly codexGuiTokenUsage = new Map<
        string,
        CodexGuiTokenUsageState | null
    >();
    private readonly codexGuiTokenUsageStreams = new Map<
        string,
        Subject<CodexGuiTokenUsageState | null>
    >();
    private readonly codexGuiRequest = new Map<string, CodexGuiRequestState | null>();
    private readonly codexGuiRequestStreams = new Map<
        string,
        Subject<CodexGuiRequestState | null>
    >();
    private readonly codexGuiSelectedModel = new Map<string, string>();
    private readonly codexGuiSelectedEffort = new Map<string, string>();
    private readonly codexGuiModelCapabilities = new Map<
        string,
        CodexGuiModelCapability[]
    >();
    private readonly unlistenFns: UnlistenFn[] = [];

    constructor(private readonly zone: NgZone) {
        this.registerEventListeners();
        window.addEventListener("unload", () => this.teardown());
    }

    clearAll(): void {
        this.codexGuiMessages.clear();
        this.codexGuiMessageStreams.clear();
        this.codexGuiHydrated.clear();
        this.codexGuiHydratedStreams.clear();
        this.codexGuiActivity.clear();
        this.codexGuiActivityStreams.clear();
        this.codexGuiPlan.clear();
        this.codexGuiPlanStreams.clear();
        this.codexGuiTokenUsage.clear();
        this.codexGuiTokenUsageStreams.clear();
        this.codexGuiRequest.clear();
        this.codexGuiRequestStreams.clear();
        this.codexGuiSelectedModel.clear();
        this.codexGuiSelectedEffort.clear();
        this.codexGuiModelCapabilities.clear();
    }

    resetTask(taskId: string): void {
        if (!taskId) {
            return;
        }
        this.setCodexGuiHydrated(taskId, false);
        this.setCodexGuiActivity(taskId, null);
        this.setCodexGuiPlan(taskId, null);
        this.setCodexGuiRequest(taskId, null);
    }

    removeTask(taskId: string): void {
        this.codexGuiMessages.delete(taskId);
        this.codexGuiMessageStreams.delete(taskId);
        this.codexGuiHydrated.delete(taskId);
        this.codexGuiHydratedStreams.delete(taskId);
        this.codexGuiActivity.delete(taskId);
        this.codexGuiActivityStreams.delete(taskId);
        this.codexGuiPlan.delete(taskId);
        this.codexGuiPlanStreams.delete(taskId);
        this.codexGuiTokenUsage.delete(taskId);
        this.codexGuiTokenUsageStreams.delete(taskId);
        this.codexGuiRequest.delete(taskId);
        this.codexGuiRequestStreams.delete(taskId);
        this.codexGuiSelectedModel.delete(taskId);
        this.codexGuiSelectedEffort.delete(taskId);
        this.codexGuiModelCapabilities.delete(taskId);
    }

    async sendMessage(
        taskId: string,
        content: string,
        model?: string | null,
        effort?: string | null,
    ): Promise<void> {
        const text = content.trim();
        if (!text) {
            return;
        }
        const resolvedModel = (model ?? this.getModel(taskId)).trim();
        const resolvedEffort = (effort ?? this.getEffort(taskId)).trim();
        this.setTrimmedValue(this.codexGuiSelectedModel, taskId, resolvedModel);
        this.setTrimmedValue(this.codexGuiSelectedEffort, taskId, resolvedEffort);
        this.pushCodexGuiMessage(taskId, {
            id: this.newMessageId(),
            role: "user",
            content: text,
            createdAt: new Date().toISOString(),
            status: "complete",
        });
        try {
            await tauriInvoke<void>(this.zone, "task_codex_gui_send", {
                req: {
                    taskId,
                    content: text,
                    model: resolvedModel || undefined,
                    effort: resolvedEffort || undefined,
                },
            });
        } catch (error) {
            const detail = error instanceof Error ? error.message : String(error);
            this.pushCodexGuiMessage(taskId, {
                id: this.newMessageId(),
                role: "system",
                content: `Codex GUI send failed: ${detail}`,
                createdAt: new Date().toISOString(),
                status: "error",
            });
            throw error;
        }
    }

    interruptTask(taskId: string): Promise<void> {
        return tauriInvoke<void>(this.zone, "task_codex_gui_interrupt", {
            req: { taskId },
        });
    }

    respondToRequest(
        taskId: string,
        requestId: string,
        response: unknown,
    ): Promise<void> {
        return tauriInvoke<void>(this.zone, "task_codex_gui_request_respond", {
            req: { taskId, requestId, response },
        });
    }

    compactThread(taskId: string): Promise<void> {
        return tauriInvoke<void>(this.zone, "task_codex_gui_compact", {
            req: { taskId },
        });
    }

    async newChat(taskId: string): Promise<void> {
        await tauriInvoke<void>(this.zone, "task_codex_gui_new_chat", {
            req: { taskId },
        });
        this.replaceCodexGuiMessages(taskId, []);
        this.setCodexGuiHydrated(taskId, true);
        this.setCodexGuiActivity(taskId, null);
        this.setCodexGuiPlan(taskId, null);
        this.setCodexGuiTokenUsage(taskId, null);
        this.setCodexGuiRequest(taskId, null);
    }

    async rollbackThread(taskId: string, numTurns = 1): Promise<void> {
        const response = await tauriInvoke<CodexGuiRollbackResponse>(
            this.zone,
            "task_codex_gui_rollback",
            { req: { taskId, numTurns } },
        );
        this.setCodexGuiPlan(taskId, null);
        this.replaceCodexGuiMessages(
            taskId,
            (response.events ?? []).map((event) => ({
                id: event.messageId,
                role: event.role as Message["role"],
                content: event.content,
                createdAt: new Date().toISOString(),
                status: event.isFinal ? "complete" : "streaming",
            })),
        );
        this.setCodexGuiHydrated(taskId, true);
    }

    messages$(taskId: string): Observable<Message[]> {
        return this.ensureCodexGuiMessageStream(taskId).asObservable();
    }

    hydrated$(taskId: string): Observable<boolean> {
        return this.ensureCodexGuiHydratedStream(taskId).asObservable();
    }

    activity$(taskId: string): Observable<CodexGuiActivityState | null> {
        return this.ensureCodexGuiActivityStream(taskId).asObservable();
    }

    plan$(taskId: string): Observable<CodexGuiPlanState | null> {
        return this.ensureCodexGuiPlanStream(taskId).asObservable();
    }

    tokenUsage$(taskId: string): Observable<CodexGuiTokenUsageState | null> {
        return this.ensureCodexGuiTokenUsageStream(taskId).asObservable();
    }

    request$(taskId: string): Observable<CodexGuiRequestState | null> {
        return this.ensureCodexGuiRequestStream(taskId).asObservable();
    }

    getMessages(taskId: string): Message[] {
        return this.codexGuiMessages.get(taskId) ?? [];
    }

    isHydrated(taskId: string): boolean {
        return this.codexGuiHydrated.get(taskId) ?? false;
    }

    getActivity(taskId: string): CodexGuiActivityState | null {
        return this.codexGuiActivity.get(taskId) ?? null;
    }

    getPlan(taskId: string): CodexGuiPlanState | null {
        return this.codexGuiPlan.get(taskId) ?? null;
    }

    getTokenUsage(taskId: string): CodexGuiTokenUsageState | null {
        return this.codexGuiTokenUsage.get(taskId) ?? null;
    }

    getRequest(taskId: string): CodexGuiRequestState | null {
        return this.codexGuiRequest.get(taskId) ?? null;
    }

    getModel(taskId: string): string {
        return this.codexGuiSelectedModel.get(taskId) ?? "";
    }

    setModel(taskId: string, model: string): void {
        this.setTrimmedValue(this.codexGuiSelectedModel, taskId, model);
    }

    async getAvailableModels(taskId: string): Promise<string[]> {
        if (!taskId) {
            return [];
        }
        const response = await tauriInvoke<CodexGuiModelsResponse>(
            this.zone,
            "task_codex_gui_models",
            { req: { taskId } },
        );
        this.codexGuiModelCapabilities.set(taskId, response.modelCapabilities ?? []);
        this.setTrimmedValue(
            this.codexGuiSelectedModel,
            taskId,
            response.selectedModel ?? "",
        );
        this.setTrimmedValue(
            this.codexGuiSelectedEffort,
            taskId,
            response.selectedEffort ?? "",
        );
        return response.models ?? [];
    }

    getEffort(taskId: string): string {
        return this.codexGuiSelectedEffort.get(taskId) ?? "";
    }

    setEffort(taskId: string, effort: string): void {
        this.setTrimmedValue(this.codexGuiSelectedEffort, taskId, effort);
    }

    getModelEfforts(taskId: string, model: string): string[] {
        const capabilities = this.codexGuiModelCapabilities.get(taskId) ?? [];
        const selected = capabilities.find((item) => item.model === model);
        return selected?.reasoningEfforts ?? [];
    }

    async getUsage(taskId: string): Promise<UsageSnapshot | null> {
        if (!taskId) {
            return null;
        }
        const response = await tauriInvoke<CodexGuiUsageResponse>(
            this.zone,
            "task_codex_gui_usage",
            { req: { taskId } },
        );
        return this.parseCodexGuiUsage(response);
    }

    async getLimitStatus(taskId: string): Promise<LimitStatus | null> {
        if (!taskId) {
            return null;
        }
        const response = await tauriInvoke<CodexGuiUsageResponse>(
            this.zone,
            "task_codex_gui_usage",
            { req: { taskId } },
        );
        return this.parseCodexGuiLimitStatus(response.rateLimits);
    }

    private registerEventListeners(): void {
        void tauriListen<MessageEvent>(
            this.zone,
            "task_codex_gui_message",
            (event) => {
                this.applyCodexGuiEvent(event.payload);
            },
        ).then((unlisten) => this.unlistenFns.push(unlisten));

        void tauriListen<HydratedEvent>(
            this.zone,
            "task_codex_gui_hydrated",
            (event) => {
                this.setCodexGuiHydrated(event.payload.taskId, true);
            },
        ).then((unlisten) => this.unlistenFns.push(unlisten));

        void tauriListen<ActivityEvent>(
            this.zone,
            "task_codex_gui_activity",
            (event) => {
                this.applyCodexGuiActivity(event.payload);
            },
        ).then((unlisten) => this.unlistenFns.push(unlisten));

        void tauriListen<PlanEvent>(
            this.zone,
            "task_codex_gui_plan",
            (event) => {
                this.applyCodexGuiPlan(event.payload);
            },
        ).then((unlisten) => this.unlistenFns.push(unlisten));

        void tauriListen<TokenUsageEvent>(
            this.zone,
            "task_codex_gui_token_usage",
            (event) => {
                this.applyCodexGuiTokenUsage(event.payload);
            },
        ).then((unlisten) => this.unlistenFns.push(unlisten));

        void tauriListen<RequestEvent>(
            this.zone,
            "task_codex_gui_request",
            (event) => {
                this.applyCodexGuiRequest(event.payload);
            },
        ).then((unlisten) => this.unlistenFns.push(unlisten));
    }

    private ensureCodexGuiMessageStream(
        taskId: string,
    ): Subject<Message[]> {
        let stream = this.codexGuiMessageStreams.get(taskId);
        if (!stream) {
            stream = new Subject<Message[]>();
            this.codexGuiMessageStreams.set(taskId, stream);
        }
        return stream;
    }

    private ensureCodexGuiHydratedStream(taskId: string): Subject<boolean> {
        let stream = this.codexGuiHydratedStreams.get(taskId);
        if (!stream) {
            stream = new Subject<boolean>();
            this.codexGuiHydratedStreams.set(taskId, stream);
        }
        return stream;
    }

    private ensureCodexGuiActivityStream(
        taskId: string,
    ): Subject<CodexGuiActivityState | null> {
        let stream = this.codexGuiActivityStreams.get(taskId);
        if (!stream) {
            stream = new Subject<CodexGuiActivityState | null>();
            this.codexGuiActivityStreams.set(taskId, stream);
        }
        return stream;
    }

    private ensureCodexGuiPlanStream(
        taskId: string,
    ): Subject<CodexGuiPlanState | null> {
        let stream = this.codexGuiPlanStreams.get(taskId);
        if (!stream) {
            stream = new Subject<CodexGuiPlanState | null>();
            this.codexGuiPlanStreams.set(taskId, stream);
        }
        return stream;
    }

    private ensureCodexGuiTokenUsageStream(
        taskId: string,
    ): Subject<CodexGuiTokenUsageState | null> {
        let stream = this.codexGuiTokenUsageStreams.get(taskId);
        if (!stream) {
            stream = new Subject<CodexGuiTokenUsageState | null>();
            this.codexGuiTokenUsageStreams.set(taskId, stream);
        }
        return stream;
    }

    private ensureCodexGuiRequestStream(
        taskId: string,
    ): Subject<CodexGuiRequestState | null> {
        let stream = this.codexGuiRequestStreams.get(taskId);
        if (!stream) {
            stream = new Subject<CodexGuiRequestState | null>();
            this.codexGuiRequestStreams.set(taskId, stream);
        }
        return stream;
    }

    private setCodexGuiHydrated(taskId: string, hydrated: boolean): void {
        if (!taskId) {
            return;
        }
        this.codexGuiHydrated.set(taskId, hydrated);
        this.ensureCodexGuiHydratedStream(taskId).next(hydrated);
    }

    private setCodexGuiPlan(taskId: string, plan: CodexGuiPlanState | null): void {
        if (!taskId) {
            return;
        }
        if (plan) {
            this.codexGuiPlan.set(taskId, plan);
        } else {
            this.codexGuiPlan.delete(taskId);
        }
        this.ensureCodexGuiPlanStream(taskId).next(plan);
    }

    private setCodexGuiTokenUsage(
        taskId: string,
        usage: CodexGuiTokenUsageState | null,
    ): void {
        if (!taskId) {
            return;
        }
        if (usage) {
            this.codexGuiTokenUsage.set(taskId, usage);
        } else {
            this.codexGuiTokenUsage.delete(taskId);
        }
        this.ensureCodexGuiTokenUsageStream(taskId).next(usage);
    }

    private setCodexGuiRequest(
        taskId: string,
        request: CodexGuiRequestState | null,
    ): void {
        if (!taskId) {
            return;
        }
        if (request) {
            this.codexGuiRequest.set(taskId, request);
        } else {
            this.codexGuiRequest.delete(taskId);
        }
        this.ensureCodexGuiRequestStream(taskId).next(request);
    }

    private applyCodexGuiActivity(event: ActivityEvent): void {
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
        this.setCodexGuiActivity(event.taskId, activity);
    }

    private applyCodexGuiPlan(event: PlanEvent): void {
        const explanation = event.explanation?.trim() || null;
        const steps = (event.plan ?? []).filter((item) => item.step?.trim().length > 0);
        this.setCodexGuiPlan(
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

    private applyCodexGuiTokenUsage(event: TokenUsageEvent): void {
        this.setCodexGuiTokenUsage(event.taskId, event.usage ?? null);
    }

    private applyCodexGuiRequest(event: RequestEvent): void {
        const kind = event.kind?.trim() ?? "";
        this.setCodexGuiRequest(
            event.taskId,
            kind && kind !== "none" && event.requestId
                ? {
                      ...event,
                      kind,
                  }
                : null,
        );
    }

    private setCodexGuiActivity(
        taskId: string,
        activity: CodexGuiActivityState | null,
    ): void {
        if (!taskId) {
            return;
        }
        if (activity) {
            this.codexGuiActivity.set(taskId, activity);
        } else {
            this.codexGuiActivity.delete(taskId);
        }
        this.ensureCodexGuiActivityStream(taskId).next(activity);
    }

    private pushCodexGuiMessage(taskId: string, message: Message): void {
        if (!taskId) {
            return;
        }
        const current = this.codexGuiMessages.get(taskId) ?? [];
        const updated = [...current, message];
        this.codexGuiMessages.set(taskId, updated);
        this.ensureCodexGuiMessageStream(taskId).next(updated);
    }

    private replaceCodexGuiMessages(taskId: string, messages: Message[]): void {
        this.codexGuiMessages.set(taskId, messages);
        this.ensureCodexGuiMessageStream(taskId).next(messages);
    }

    private applyCodexGuiEvent(event: MessageEvent): void {
        const current = this.codexGuiMessages.get(event.taskId) ?? [];
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
                status: event.isFinal ? "complete" : existing.status,
            };
            const updated = [...current];
            updated[existingIndex] = updatedMessage;
            this.codexGuiMessages.set(event.taskId, updated);
            this.ensureCodexGuiMessageStream(event.taskId).next(updated);
            return;
        }

        if (!hasIncomingContent) {
            return;
        }

        this.pushCodexGuiMessage(event.taskId, {
            id: event.messageId,
            role: event.role,
            content: incomingContent,
            createdAt: new Date().toISOString(),
            status: event.isFinal ? "complete" : "streaming",
        });
    }

    private parseCodexGuiUsage(response: CodexGuiUsageResponse): UsageSnapshot | null {
        const envelope = this.codexGuiRateLimitEnvelope(response.rateLimits);
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

    private parseCodexGuiLimitStatus(rateLimits: unknown): LimitStatus | null {
        const envelope = this.codexGuiRateLimitEnvelope(rateLimits);
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

    private codexGuiRateLimitEnvelope(
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
}

type CodexGuiModelsResponse = {
    models: string[];
    modelCapabilities: CodexGuiModelCapability[];
    selectedModel?: string | null;
    selectedEffort?: string | null;
};

type CodexGuiUsageResponse = {
    rateLimits?: unknown;
    windowDurationHours?: number | null;
    workingPeriods?: WorkingPeriod[];
};

type CodexGuiModelCapability = {
    model: string;
    reasoningEfforts: string[];
};

type CodexGuiRollbackResponse = {
    events: Array<{
        messageId: string;
        role: string;
        content: string;
        isDelta: boolean;
        isFinal: boolean;
    }>;
};
