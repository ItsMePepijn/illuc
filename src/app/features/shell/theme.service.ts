import { Injectable, NgZone } from "@angular/core";
import { tauriInvoke } from "../../shared/tauri/tauri-zone";

type ThemeSettings = Record<string, string>;
type ThemeSettingsResponse = {
    syntaxTheme: string;
    values: ThemeSettings;
};

@Injectable({
    providedIn: "root",
})
export class ThemeService {
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
