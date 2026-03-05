import { CommonModule } from "@angular/common";
import {
    ChangeDetectionStrategy,
    ChangeDetectorRef,
    Component,
    Input,
    OnChanges,
    OnDestroy,
    SimpleChanges,
} from "@angular/core";
import { ThrobberComponent } from "../throbber/throbber.component";

@Component({
    selector: "app-codex-gui-typing-indicator",
    standalone: true,
    imports: [CommonModule, ThrobberComponent],
    templateUrl: "./typing-indicator.component.html",
    styleUrl: "./typing-indicator.component.css",
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TypingIndicatorComponent implements OnChanges, OnDestroy {
    @Input() startedAt: string | null = null;
    @Input() label = "Working";
    @Input() showLabel = false;

    elapsedLabel = "";

    private intervalId: number | null = null;

    constructor(private readonly cdr: ChangeDetectorRef) {}

    ngOnChanges(changes: SimpleChanges): void {
        if (changes["startedAt"] || changes["showLabel"]) {
            this.syncElapsedTimer();
        }
    }

    ngOnDestroy(): void {
        this.clearElapsedTimer();
    }

    get displayLabel(): string {
        if (!this.showLabel) {
            return this.label;
        }
        if (!this.elapsedLabel) {
            return this.label;
        }
        return `${this.label} (${this.elapsedLabel} • esc to interrupt)`;
    }

    private syncElapsedTimer(): void {
        this.clearElapsedTimer();
        this.updateElapsedLabel();
        if (!this.showLabel || !this.startedAt) {
            return;
        }
        this.intervalId = window.setInterval(() => {
            this.updateElapsedLabel();
        }, 1000);
    }

    private clearElapsedTimer(): void {
        if (this.intervalId !== null) {
            window.clearInterval(this.intervalId);
            this.intervalId = null;
        }
    }

    private updateElapsedLabel(): void {
        this.elapsedLabel = formatElapsed(this.startedAt);
        this.cdr.markForCheck();
    }
}

function formatElapsed(startedAt: string | null): string {
    if (!startedAt) {
        return "";
    }
    const startedMs = Date.parse(startedAt);
    if (Number.isNaN(startedMs)) {
        return "";
    }
    const elapsedSeconds = Math.max(0, Math.floor((Date.now() - startedMs) / 1000));
    const minutes = Math.floor(elapsedSeconds / 60);
    const seconds = elapsedSeconds % 60;
    if (minutes <= 0) {
        return `${seconds}s`;
    }
    return `${minutes}m ${seconds.toString().padStart(2, "0")}s`;
}
