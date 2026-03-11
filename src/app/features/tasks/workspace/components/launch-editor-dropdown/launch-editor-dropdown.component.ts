import { CommonModule } from "@angular/common";
import {
    ChangeDetectorRef,
    Component,
    HostListener,
    Input,
    NgZone,
    OnInit,
} from "@angular/core";
import { LauncherService } from "../../../../launcher/launcher.service";
import { EditorApp } from "../../../../launcher/models";
import { LoadingButtonComponent } from "../../../../../shared/components/loading-button/loading-button.component";
import { TaskStore } from "../../../task.store";
import { IconCodeBracketsComponent } from "../icon-code-brackets/icon-code-brackets.component";
import { EditorAppIconComponent } from "../editor-app-icon/editor-app-icon.component";

@Component({
    selector: "app-launch-editor-dropdown",
    standalone: true,
    imports: [
        CommonModule,
        LoadingButtonComponent,
        IconCodeBracketsComponent,
        EditorAppIconComponent,
    ],
    templateUrl: "./launch-editor-dropdown.component.html",
    styleUrl: "./launch-editor-dropdown.component.css",
})
export class LaunchEditorDropdownComponent implements OnInit {
    @Input() path: string | null = null;
    @Input() title = "Launch in editor";
    @Input() ariaLabel = "Launch in editor";
    @Input() buttonClass = "";
    @Input() compact = false;
    @Input() taskId: string | null = null;

    editors: EditorApp[] = [];
    menuOpen = false;
    isLoadingEditors = false;
    launchingEditorId: string | null = null;

    constructor(
        private readonly launcher: LauncherService,
        private readonly taskStore: TaskStore,
        private readonly zone: NgZone,
        private readonly cdr: ChangeDetectorRef,
    ) {}

    get isBusy(): boolean {
        return this.isLoadingEditors || this.launchingEditorId !== null;
    }

    get isDisabled(): boolean {
        return !this.path || this.isBusy || this.editors.length === 0;
    }

    get resolvedButtonClass(): string {
        return [
            this.compact ? "" : "action-btn action-text-btn",
            this.buttonClass,
        ]
            .filter(Boolean)
            .join(" ");
    }

    ngOnInit(): void {
        void this.loadEditors();
    }

    async loadEditors(): Promise<void> {
        this.isLoadingEditors = true;
        try {
            this.editors = await this.launcher.listInstalledEditors();
        } catch (error) {
            console.error("Failed to list installed editors", error);
            this.editors = [];
        } finally {
            this.zone.run(() => {
                this.isLoadingEditors = false;
                this.cdr.markForCheck();
            });
        }
    }

    toggleMenu(event: MouseEvent): void {
        event.stopPropagation();
        if (this.isDisabled) {
            this.menuOpen = false;
            return;
        }
        this.menuOpen = !this.menuOpen;
    }

    async choose(editor: EditorApp, event: MouseEvent): Promise<void> {
        event.stopPropagation();
        if (!this.path || this.isBusy) {
            return;
        }

        this.menuOpen = false;
        this.launchingEditorId = editor.id;

        try {
            await this.launcher.openInEditor(this.path, editor.id);
            if (this.taskId) {
                this.taskStore.rememberEditorForTask(this.taskId, editor.id);
            }
        } catch (error) {
            console.error(`Failed to launch ${editor.name}`, error);
        } finally {
            this.zone.run(() => {
                this.launchingEditorId = null;
                this.cdr.markForCheck();
            });
        }
    }

    @HostListener("document:click")
    handleDocumentClick(): void {
        this.menuOpen = false;
    }

    @HostListener("document:keydown.escape", ["$event"])
    handleEscape(event: Event): void {
        if (!this.menuOpen) {
            return;
        }
        event.preventDefault();
        this.menuOpen = false;
    }
}
