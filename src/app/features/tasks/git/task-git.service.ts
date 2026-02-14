import { Injectable, NgZone } from "@angular/core";
import { tauriInvoke } from "../../../shared/tauri/tauri-zone";

@Injectable({
    providedIn: "root",
})
export class TaskGitService {
    constructor(private readonly zone: NgZone) {}

    async listBranches(baseRepoPath: string): Promise<string[]> {
        return tauriInvoke<string[]>(this.zone, "task_git_list_branches", {
            path: baseRepoPath,
        });
    }
}
