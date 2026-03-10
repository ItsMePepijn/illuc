import { bootstrapApplication } from "@angular/platform-browser";
import { appConfig } from "./app/features/shell/app.config";
import { RootComponent } from "./app/features/shell/components/root/root.component";
import { AgentChatStore } from "./app/features/tasks/agent-chat/agent-chat.store";
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
                agentChatStore: appRef.injector
                    .get(AgentChatStore)
                    .snapshotDevState(),
            });
            appRef.destroy();
        });

        return appRef;
    })
    .catch((err) => console.error(err));
