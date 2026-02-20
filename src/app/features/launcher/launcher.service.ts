import { Injectable, NgZone } from "@angular/core";
import { tauriInvoke } from "../../shared/tauri/tauri-zone";

@Injectable({
    providedIn: "root",
})
export class LauncherService {
    constructor(private readonly zone: NgZone) {}

    openInVsCode(path: string): Promise<void> {
        return tauriInvoke<void>(this.zone, "open_path_in_vscode", { path });
    }

    openFileInVsCode(
        path: string,
        line?: number,
        column?: number,
    ): Promise<void> {
        return tauriInvoke<void>(this.zone, "open_file_in_vscode", {
            req: { path, line, column },
        });
    }

    openTerminal(path: string): Promise<void> {
        return tauriInvoke<void>(this.zone, "open_path_terminal", { path });
    }

    openInExplorer(path: string): Promise<void> {
        return tauriInvoke<void>(this.zone, "open_path_in_explorer", { path });
    }

    openSettingsInVsCode(): Promise<void> {
        return tauriInvoke<void>(this.zone, "settings_open_in_vscode");
    }
}
