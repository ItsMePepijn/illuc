import { Injectable, NgZone } from "@angular/core";
import { EditorApp } from "./models";
import { tauriInvoke } from "../../shared/tauri/tauri-zone";

@Injectable({
    providedIn: "root",
})
export class LauncherService {
    private editorsPromise: Promise<EditorApp[]> | null = null;

    constructor(private readonly zone: NgZone) {}

    listInstalledEditors(forceRefresh = false): Promise<EditorApp[]> {
        if (!this.editorsPromise || forceRefresh) {
            this.editorsPromise = tauriInvoke<EditorApp[]>(
                this.zone,
                "list_installed_editors",
            ).catch((error) => {
                this.editorsPromise = null;
                throw error;
            });
        }

        return this.editorsPromise;
    }

    openInEditor(path: string, editorId: string): Promise<void> {
        return tauriInvoke<void>(this.zone, "open_path_in_editor", {
            req: { path, editorId },
        });
    }

    openFileInEditor(
        path: string,
        editorId: string,
        line?: number,
        column?: number,
    ): Promise<void> {
        return tauriInvoke<void>(this.zone, "open_file_in_editor", {
            req: { path, editorId, line, column },
        });
    }

    openInDefaultEditor(path: string): Promise<void> {
        return tauriInvoke<void>(this.zone, "open_path_in_default_editor", { path });
    }

    openFileInDefaultEditor(
        path: string,
        line?: number,
        column?: number,
    ): Promise<void> {
        return tauriInvoke<void>(this.zone, "open_file_in_default_editor", {
            req: { path, line, column },
        });
    }

    openTerminal(path: string): Promise<void> {
        return tauriInvoke<void>(this.zone, "open_path_terminal", { path });
    }

    openInExplorer(path: string): Promise<void> {
        return tauriInvoke<void>(this.zone, "open_path_in_explorer", { path });
    }

    openSettingsInDefaultEditor(): Promise<void> {
        return tauriInvoke<void>(this.zone, "settings_open_in_default_editor");
    }
}
