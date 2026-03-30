import { Injectable, NgZone, signal } from "@angular/core";
import { tauriInvoke } from "../../shared/tauri/tauri-zone";
import { TokenUsagePayload } from "./models";

const EMPTY_USAGE: TokenUsagePayload = {
    version: 1,
    currency: "USD",
    pricingVersion: 0,
    pricingSourceUrl: "",
    pricingPublishedAt: "",
    note: "",
    scopes: {
        global: {
            totals: {
                inputTokens: 0,
                cachedInputTokens: 0,
                outputTokens: 0,
                totalTokens: 0,
                inputCost: 0,
                cachedInputCost: 0,
                outputCost: 0,
                totalCost: 0,
            },
            sessionCount: 0,
            byDay: [],
            byMonthSessionCounts: {},
        },
        workspace: {
            totals: {
                inputTokens: 0,
                cachedInputTokens: 0,
                outputTokens: 0,
                totalTokens: 0,
                inputCost: 0,
                cachedInputCost: 0,
                outputCost: 0,
                totalCost: 0,
            },
            sessionCount: 0,
            byDay: [],
            byMonthSessionCounts: {},
        },
    },
    byTask: [],
    unknownPricedModels: [],
};

@Injectable({
    providedIn: "root",
})
export class TokenUsageService {
    private readonly usageSignal = signal<TokenUsagePayload | null>(null);
    private readonly loadingSignal = signal(false);
    readonly usage = this.usageSignal.asReadonly();
    readonly loading = this.loadingSignal.asReadonly();

    private baseRepoPath: string | null = null;

    constructor(private readonly zone: NgZone) {}

    async syncBaseRepo(baseRepoPath: string | null): Promise<void> {
        if (baseRepoPath === this.baseRepoPath) {
            return;
        }
        this.baseRepoPath = baseRepoPath;
        if (!baseRepoPath) {
            this.zone.run(() => {
                this.loadingSignal.set(false);
                this.usageSignal.set(null);
            });
            return;
        }
        this.zone.run(() => {
            this.loadingSignal.set(true);
            this.usageSignal.set(null);
        });
        await this.loadUsage(baseRepoPath);
    }

    async refresh(): Promise<void> {
        if (!this.baseRepoPath) {
            return;
        }
        this.zone.run(() => {
            this.loadingSignal.set(true);
            this.usageSignal.set(null);
        });
        await this.loadUsage(this.baseRepoPath);
    }

    private async loadUsage(baseRepoPath: string): Promise<void> {
        try {
            const payload = await tauriInvoke<TokenUsagePayload>(
                this.zone,
                "token_usage_get",
                { req: { baseRepoPath } },
                120_000,
            );
            this.zone.run(() => {
                this.loadingSignal.set(false);
                this.usageSignal.set(this.normalizePayload(payload));
            });
        } catch (error) {
            console.error("Failed to load token usage data", error);
            this.zone.run(() => {
                this.loadingSignal.set(false);
                this.usageSignal.set({ ...EMPTY_USAGE });
            });
        }
    }

    private normalizePayload(payload: TokenUsagePayload): TokenUsagePayload {
        if (!payload || typeof payload !== "object") {
            return { ...EMPTY_USAGE };
        }
        return {
            version: payload.version ?? EMPTY_USAGE.version,
            currency: payload.currency ?? EMPTY_USAGE.currency,
            pricingVersion:
                payload.pricingVersion ?? EMPTY_USAGE.pricingVersion,
            pricingSourceUrl:
                payload.pricingSourceUrl ?? EMPTY_USAGE.pricingSourceUrl,
            pricingPublishedAt:
                payload.pricingPublishedAt ?? EMPTY_USAGE.pricingPublishedAt,
            note: payload.note ?? EMPTY_USAGE.note,
            scopes: {
                global: {
                    totals:
                        payload.scopes?.global?.totals ??
                        EMPTY_USAGE.scopes.global.totals,
                    sessionCount:
                        payload.scopes?.global?.sessionCount ??
                        EMPTY_USAGE.scopes.global.sessionCount,
                    byDay:
                        payload.scopes?.global?.byDay ??
                        EMPTY_USAGE.scopes.global.byDay,
                    byMonthSessionCounts:
                        payload.scopes?.global?.byMonthSessionCounts ??
                        EMPTY_USAGE.scopes.global.byMonthSessionCounts,
                },
                workspace: {
                    totals:
                        payload.scopes?.workspace?.totals ??
                        EMPTY_USAGE.scopes.workspace.totals,
                    sessionCount:
                        payload.scopes?.workspace?.sessionCount ??
                        EMPTY_USAGE.scopes.workspace.sessionCount,
                    byDay:
                        payload.scopes?.workspace?.byDay ??
                        EMPTY_USAGE.scopes.workspace.byDay,
                    byMonthSessionCounts:
                        payload.scopes?.workspace?.byMonthSessionCounts ??
                        EMPTY_USAGE.scopes.workspace.byMonthSessionCounts,
                },
            },
            byTask: payload.byTask ?? [],
            unknownPricedModels: payload.unknownPricedModels ?? [],
        };
    }
}
