import { Injectable, NgZone } from "@angular/core";
import { tauriInvoke } from "../../shared/tauri/tauri-zone";

@Injectable({
    providedIn: "root",
})
export class LauncherService {
    constructor(private readonly zone: NgZone) {}

    async openInVsCode(path: string): Promise<void> {
        await tauriInvoke<void>(this.zone, "open_path_in_vscode", { path });
    }

    async openFileInVsCode(
        path: string,
        line?: number,
        column?: number,
    ): Promise<void> {
        await tauriInvoke<void>(this.zone, "open_file_in_vscode", {
            req: { path, line, column },
        });
    }

    async openTerminal(path: string): Promise<void> {
        await tauriInvoke<void>(this.zone, "open_path_terminal", { path });
    }

    async openInExplorer(path: string): Promise<void> {
        await tauriInvoke<void>(this.zone, "open_path_in_explorer", { path });
    }
}
