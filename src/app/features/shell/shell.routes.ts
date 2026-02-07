import { Routes, UrlMatchResult, UrlSegment } from "@angular/router";
import { AppComponent } from "./components/app/app.component";

function shellMatcher(segments: UrlSegment[]): UrlMatchResult | null {
    if (segments.length === 0) {
        return { consumed: [] };
    }
    if (segments.length === 1) {
        const [first] = segments;
        if (
            first.path === "dashboard" ||
            first.path === "getting-started"
        ) {
            return { consumed: segments };
        }
        return null;
    }
    if (segments.length === 2 && segments[0].path === "tasks") {
        return {
            consumed: segments,
            posParams: {
                taskId: segments[1],
            },
        };
    }
    return null;
}

export const SHELL_ROUTES: Routes = [
    { matcher: shellMatcher, component: AppComponent },
    { path: "**", redirectTo: "dashboard" },
];
