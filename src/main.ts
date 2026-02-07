import { bootstrapApplication } from "@angular/platform-browser";
import { appConfig } from "./app/features/shell/app.config";
import { RootComponent } from "./app/features/shell/components/root/root.component";

bootstrapApplication(RootComponent, appConfig).catch((err) =>
    console.error(err),
);
