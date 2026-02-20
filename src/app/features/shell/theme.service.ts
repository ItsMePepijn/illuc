import { Injectable, NgZone } from "@angular/core";
import { type UnlistenFn } from "@tauri-apps/api/event";
import { tauriInvoke, tauriListen } from "../../shared/tauri/tauri-zone";

type ThemeSettings = Record<string, string>;
type ThemeSettingsResponse = {
    syntaxTheme: string;
    values: ThemeSettings;
};

@Injectable({
    providedIn: "root",
})
export class ThemeService {
    private static readonly settingsThemeChangedEvent =
        "settings_theme_changed";
    private settingsThemeChangedUnlisten?: UnlistenFn;
    private pendingReloadTimer: number | null = null;
    private reloadInFlight = false;
    private reloadQueued = false;

    constructor(private readonly zone: NgZone) {}

    async applyFromSettings(): Promise<void> {
        try {
            const theme = await tauriInvoke<ThemeSettingsResponse>(
                this.zone,
                "settings_theme_get",
            );
            this.applyThemeVariables(theme.values);
            this.applySyntaxTheme(theme.syntaxTheme);
            // Consumers like xterm.js need to re-read computed CSS variables.
            window.dispatchEvent(new CustomEvent("illuc-theme-applied"));
        } catch (error) {
            console.warn("Failed to load theme settings.", error);
        }
    }

    async startSettingsThemeWatch(): Promise<void> {
        if (this.settingsThemeChangedUnlisten) {
            return;
        }

        try {
            this.settingsThemeChangedUnlisten = await tauriListen<null>(
                this.zone,
                ThemeService.settingsThemeChangedEvent,
                () => {
                    this.scheduleReload();
                },
            );
        } catch (error) {
            console.warn("Failed to listen for settings/theme updates.", error);
            return;
        }

        window.addEventListener("unload", () => {
            void this.stopSettingsThemeWatch();
        });
    }

    async stopSettingsThemeWatch(): Promise<void> {
        if (this.pendingReloadTimer !== null) {
            window.clearTimeout(this.pendingReloadTimer);
            this.pendingReloadTimer = null;
        }

        if (!this.settingsThemeChangedUnlisten) {
            return;
        }

        try {
            await this.settingsThemeChangedUnlisten();
        } catch (error) {
            console.warn("Failed to stop settings/theme listener.", error);
        } finally {
            this.settingsThemeChangedUnlisten = undefined;
        }
    }

    private scheduleReload(): void {
        if (this.pendingReloadTimer !== null) {
            window.clearTimeout(this.pendingReloadTimer);
        }
        this.pendingReloadTimer = window.setTimeout(() => {
            this.pendingReloadTimer = null;
            void this.reloadTheme();
        }, 150);
    }

    private async reloadTheme(): Promise<void> {
        if (this.reloadInFlight) {
            this.reloadQueued = true;
            return;
        }

        this.reloadInFlight = true;
        this.reloadQueued = false;

        try {
            await this.applyFromSettings();
        } finally {
            this.reloadInFlight = false;
            if (this.reloadQueued) {
                this.scheduleReload();
            }
        }
    }

    private applyThemeVariables(theme: ThemeSettings): void {
        const rootStyle = document.documentElement.style;
        for (const [key, value] of Object.entries(theme)) {
            if (typeof value !== "string") {
                continue;
            }
            const normalizedKey = key.trim();
            // Keys come from TOML and become CSS variables. Keep this strict to avoid
            // surprises from arbitrary TOML keys.
            if (!/^[a-z0-9_-]+(\.[a-z0-9_-]+)+$/i.test(normalizedKey)) {
                continue;
            }
            // Use split/join instead of replaceAll for older WebViews.
            const cssKey = normalizedKey.split(".").join("-");
            rootStyle.setProperty(
                `--${cssKey}`,
                value,
            );
        }
    }

    private applySyntaxTheme(syntaxTheme: string): void {
        const normalized = (syntaxTheme || "").trim().toLowerCase();
        const value = normalized === "dark" ? "dark" : "light";
        document.documentElement.setAttribute("data-syntax-theme", value);
    }
}
