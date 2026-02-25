import { CommonModule } from "@angular/common";
import {
    ChangeDetectionStrategy,
    Component,
    ElementRef,
    EventEmitter,
    HostListener,
    Input,
    OnChanges,
    Output,
    SimpleChanges,
    ViewChild,
} from "@angular/core";
import { FormsModule } from "@angular/forms";
import { ContextStatusComponent } from "../context-status/context-status.component";

export type CodexGuiModelOption = { value: string; label: string };

@Component({
    selector: "app-codex-gui-composer",
    standalone: true,
    imports: [CommonModule, FormsModule, ContextStatusComponent],
    templateUrl: "./composer.component.html",
    styleUrl: "./composer.component.css",
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ComposerComponent implements OnChanges {
    @Input() prompt = "";
    @Input() sending = false;
    @Input() isWorking = false;
    @Input() limitWindowDurationMins: number | null = null;
    @Input() limitUsedPercent: number | null = null;
    @Input() contextWindowUsed: number | null = null;
    @Input() contextWindowSize: number | null = null;
    @Input() selectedModel = "";
    @Input() modelOptions: CodexGuiModelOption[] = [];
    @Input() selectedEffort = "";
    @Input() effortOptions: CodexGuiModelOption[] = [];

    @Output() promptChange = new EventEmitter<string>();
    @Output() sendRequested = new EventEmitter<void>();
    @Output() stopRequested = new EventEmitter<void>();
    @Output() modelChange = new EventEmitter<string>();
    @Output() effortChange = new EventEmitter<string>();

    @ViewChild("composerInput") composerInput?: ElementRef<HTMLTextAreaElement>;

    modelMenuOpen = false;
    effortMenuOpen = false;

    ngOnChanges(changes: SimpleChanges): void {
        if (changes["prompt"]) {
            this.resizeComposer();
        }
    }

    onPromptInput(value: string): void {
        this.promptChange.emit(value);
        this.resizeComposer();
    }

    onKeyDown(event: KeyboardEvent): void {
        if (event.key === "Escape" && this.isWorking) {
            event.preventDefault();
            this.stopRequested.emit();
            return;
        }
        if (event.key !== "Enter" || event.shiftKey) {
            return;
        }
        event.preventDefault();
        this.sendRequested.emit();
    }

    onSend(): void {
        this.sendRequested.emit();
    }

    onStop(): void {
        this.stopRequested.emit();
    }

    toggleModelMenu(event: MouseEvent): void {
        event.preventDefault();
        event.stopPropagation();
        this.modelMenuOpen = !this.modelMenuOpen;
    }

    selectModelOption(model: string, event: MouseEvent): void {
        event.preventDefault();
        event.stopPropagation();
        this.modelChange.emit(model);
        this.modelMenuOpen = false;
    }

    toggleEffortMenu(event: MouseEvent): void {
        event.preventDefault();
        event.stopPropagation();
        this.effortMenuOpen = !this.effortMenuOpen;
    }

    selectEffortOption(effort: string, event: MouseEvent): void {
        event.preventDefault();
        event.stopPropagation();
        this.effortChange.emit(effort);
        this.effortMenuOpen = false;
    }

    modelLabel(model: string): string {
        if (!model) {
            return "Model";
        }
        const selected = this.modelOptions.find((option) => option.value === model);
        const base = selected ? selected.label : model.replaceAll("-", " ");
        return base.replace(/\bgpt\b/g, "GPT");
    }

    effortLabel(effort: string): string {
        if (!effort) {
            return "Effort";
        }
        const selected = this.effortOptions.find((option) => option.value === effort);
        const raw = selected ? selected.label : effort;
        const normalized = raw.replaceAll("-", " ").trim();
        const mapped = /^x\s*high$/i.test(normalized)
            ? "extra high"
            : normalized;
        const base = mapped;
        return base.replace(/\b\w/g, (char) => char.toUpperCase());
    }

    effortSelectionLabel(effort: string): string {
        if (!effort) {
            return "Reasoning";
        }
        return `${this.effortLabel(effort)} reasoning`;
    }

    @HostListener("document:click")
    closeModelMenu(): void {
        this.modelMenuOpen = false;
        this.effortMenuOpen = false;
    }

    private resizeComposer(): void {
        requestAnimationFrame(() => {
            const textarea = this.composerInput?.nativeElement;
            if (!textarea) {
                return;
            }
            textarea.style.height = "0px";
            textarea.style.height = `${Math.min(textarea.scrollHeight, 200)}px`;
        });
    }

}
