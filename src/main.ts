import { bootstrapApplication } from "@angular/platform-browser";
import { appConfig } from "./app/features/shell/app.config";
import { RootComponent } from "./app/features/shell/components/root/root.component";
import { CodexGuiStore } from "./app/features/tasks/agents/codex-gui.store";
import { TaskStore } from "./app/features/tasks/task.store";
import { writeIllucHmrState } from "./app/shared/hmr/hmr-state";

bootstrapApplication(RootComponent, appConfig)
    .then((appRef) => {
        if (!import.meta.hot) {
            return appRef;
        }

        import.meta.hot.accept();
        import.meta.hot.dispose(() => {
            writeIllucHmrState({
                taskStore: appRef.injector.get(TaskStore).snapshotDevState(),
                codexGuiStore: appRef.injector
                    .get(CodexGuiStore)
                    .snapshotDevState(),
            });
            appRef.destroy();
        });

        return appRef;
    })
    .catch((err) => console.error(err));
