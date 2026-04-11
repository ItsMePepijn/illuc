import { CommonModule } from "@angular/common";
import { ConnectedPosition, OverlayModule } from "@angular/cdk/overlay";
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
import { TaskStore } from "../../../task.store";
import { IconCodeBracketsComponent } from "../icon-code-brackets/icon-code-brackets.component";
import { EditorAppIconComponent } from "../editor-app-icon/editor-app-icon.component";
import { IconFolderOpenComponent } from "../icon-folder-open/icon-folder-open.component";
import { RailButtonComponent } from "../../../view/components/rail-button/rail-button.component";

@Component({
    selector: "app-launch-editor-dropdown",
    standalone: true,
    imports: [
        CommonModule,
        OverlayModule,
        IconCodeBracketsComponent,
        EditorAppIconComponent,
        IconFolderOpenComponent,
        RailButtonComponent,
    ],
    templateUrl: "./launch-editor-dropdown.component.html",
    styleUrl: "./launch-editor-dropdown.component.css",
})
export class LaunchEditorDropdownComponent implements OnInit {
    private static readonly DEFAULT_MENU_POSITIONS: ConnectedPosition[] = [
        {
            originX: "start",
            originY: "bottom",
            overlayX: "start",
            overlayY: "top",
            offsetY: 6,
        },
        {
            originX: "end",
            originY: "bottom",
            overlayX: "end",
            overlayY: "top",
            offsetY: 6,
        },
    ];

    private static readonly COMPACT_MENU_POSITIONS: ConnectedPosition[] = [
        {
            originX: "end",
            originY: "bottom",
            overlayX: "end",
            overlayY: "top",
            offsetY: 6,
        },
        {
            originX: "start",
            originY: "bottom",
            overlayX: "start",
            overlayY: "top",
            offsetY: 6,
        },
    ];

    private readonly isWindows = navigator.userAgent
        .toLowerCase()
        .includes("windows");

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
        return !this.path || this.isBusy;
    }

    get explorerLabel(): string {
        return this.isWindows ? "Open in Explorer" : "Open in File Manager";
    }

    get menuPositions(): ConnectedPosition[] {
        return this.compact
            ? LaunchEditorDropdownComponent.COMPACT_MENU_POSITIONS
            : LaunchEditorDropdownComponent.DEFAULT_MENU_POSITIONS;
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

    toggleMenu(): void {
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

    async openInExplorer(event: MouseEvent): Promise<void> {
        event.stopPropagation();
        if (!this.path || this.isBusy) {
            return;
        }

        this.menuOpen = false;
        this.launchingEditorId = "__explorer__";

        try {
            await this.launcher.openInExplorer(this.path);
        } catch (error) {
            console.error(`Failed to ${this.explorerLabel.toLowerCase()}`, error);
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
