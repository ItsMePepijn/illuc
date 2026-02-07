import { ApplicationConfig } from "@angular/core";
import { provideRouter } from "@angular/router";
import { SHELL_ROUTES } from "./shell.routes";
export const appConfig: ApplicationConfig = {
    providers: [provideRouter(SHELL_ROUTES)],
};
