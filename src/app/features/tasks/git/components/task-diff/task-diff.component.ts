import { CommonModule } from "@angular/common";
import {
    ChangeDetectionStrategy,
    ChangeDetectorRef,
    Component,
    Input,
    OnChanges,
    OnDestroy,
    SimpleChanges,
    ViewChild,
} from "@angular/core";
import {
    CdkVirtualScrollViewport,
    ScrollingModule,
} from "@angular/cdk/scrolling";
import hljs from "highlight.js/lib/core";
import javascript from "highlight.js/lib/languages/javascript";
import json from "highlight.js/lib/languages/json";
import typescript from "highlight.js/lib/languages/typescript";
import xml from "highlight.js/lib/languages/xml";
import css from "highlight.js/lib/languages/css";
import scss from "highlight.js/lib/languages/scss";
import bash from "highlight.js/lib/languages/bash";
import python from "highlight.js/lib/languages/python";
import java from "highlight.js/lib/languages/java";
import go from "highlight.js/lib/languages/go";
import rust from "highlight.js/lib/languages/rust";
import yaml from "highlight.js/lib/languages/yaml";
import csharp from "highlight.js/lib/languages/csharp";
import c from "highlight.js/lib/languages/c";
import cpp from "highlight.js/lib/languages/cpp";
import markdown from "highlight.js/lib/languages/markdown";
import { DomSanitizer, SafeHtml } from "@angular/platform-browser";
import { Subscription } from "rxjs";
import { DiffLineType, DiffMode, DiffPayload } from "../../../task.models";
import { TaskStore } from "../../../task.store";
import { LauncherService } from "../../../../launcher/launcher.service";
import {
    FileTreeComponent,
    FileTreeNode,
} from "../file-tree/file-tree.component";

hljs.registerLanguage("javascript", javascript);
hljs.registerLanguage("typescript", typescript);
hljs.registerLanguage("json", json);
hljs.registerLanguage("xml", xml);
hljs.registerLanguage("html", xml);
hljs.registerLanguage("css", css);
hljs.registerLanguage("scss", scss);
hljs.registerLanguage("bash", bash);
hljs.registerLanguage("shell", bash);
hljs.registerLanguage("python", python);
hljs.registerLanguage("java", java);
hljs.registerLanguage("go", go);
hljs.registerLanguage("rust", rust);
hljs.registerLanguage("yaml", yaml);
hljs.registerLanguage("csharp", csharp);
hljs.registerLanguage("c", c);
hljs.registerLanguage("cpp", cpp);
hljs.registerLanguage("markdown", markdown);


interface RenderedDiffLine {
    type: DiffLineType;
    html: SafeHtml;
}

type DiffRowKind = "header" | "line" | "spacer";

interface RenderedDiffRow {
    kind: DiffRowKind;
    filePath: string;
    displayName?: string;
    status?: string;
    line?: RenderedDiffLine;
}

@Component({
    selector: "app-task-diff",
    standalone: true,
    imports: [CommonModule, FileTreeComponent, ScrollingModule],
    templateUrl: "./task-diff.component.html",
    styleUrl: "./task-diff.component.css",
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TaskDiffComponent implements OnChanges, OnDestroy {
    @Input() taskId: string | null = null;
    @Input() baseBranch: string | null = null;
    @Input() worktreePath: string | null = null;
    @ViewChild(CdkVirtualScrollViewport)
    diffViewport?: CdkVirtualScrollViewport;

    diffPayload: DiffPayload | null = null;
    renderedRows: RenderedDiffRow[] = [];
    private rowIndexByPath = new Map<string, number>();
    fileTree: FileTreeNode[] = [];
    lastUpdated: Date | null = null;
    error: string | null = null;
    isLoading = false;
    hasLoaded = false;
    diffMode: DiffMode = "worktree";
    readonly rowHeight = 28;
    readonly spacerHeight = 12;
    private diffSubscription?: Subscription;
    private diffWatchStop?: () => Promise<void>;
    private watchVersion = 0;

    constructor(
        private readonly taskStore: TaskStore,
        private readonly sanitizer: DomSanitizer,
        private readonly cdr: ChangeDetectorRef,
        private readonly launcher: LauncherService,
    ) {}

    ngOnChanges(changes: SimpleChanges): void {
        if (changes["taskId"]) {
            void this.restartDiffWatch();
        }
    }

    ngOnDestroy(): void {
        void this.stopDiffWatch();
    }

    setDiffMode(mode: DiffMode): void {
        if (this.diffMode === mode) {
            return;
        }
        this.diffMode = mode;
        this.diffPayload = null;
        this.renderedRows = [];
        this.rowIndexByPath = new Map();
        this.fileTree = [];
        this.error = null;
        this.isLoading = false;
        this.hasLoaded = false;
        void this.restartDiffWatch();
    }

    private async restartDiffWatch(): Promise<void> {
        const watchVersion = ++this.watchVersion;
        await this.stopDiffWatch();
        if (this.watchVersion !== watchVersion) {
            return;
        }
        this.diffPayload = null;
        this.renderedRows = [];
        this.rowIndexByPath = new Map();
        this.fileTree = [];
        this.error = null;
        this.isLoading = false;
        this.hasLoaded = false;
        if (!this.taskId) {
            return;
        }
        this.isLoading = true;
        this.hasLoaded = false;
        this.cdr.detectChanges();
        const handle = this.taskStore.watchDiff(this.taskId, this.diffMode);
        this.diffWatchStop = handle.stop;
        this.diffSubscription = handle.state$.subscribe((state) => {
            if (this.watchVersion !== watchVersion) {
                return;
            }
            if (state.payload && state.payload.taskId !== this.taskId) {
                return;
            }
            this.diffPayload = state.payload;
            this.renderedRows = state.payload
                ? this.buildRenderedRows(state.payload)
                : [];
            this.fileTree = state.payload
                ? this.buildFileTree(state.payload.files)
                : [];
            this.lastUpdated = state.lastUpdated;
            this.error = state.error;
            this.isLoading = state.isLoading;
            this.hasLoaded = state.hasLoaded;
            this.cdr.detectChanges();
            this.diffViewport?.checkViewportSize();
        });
    }

    private async stopDiffWatch(): Promise<void> {
        this.diffSubscription?.unsubscribe();
        this.diffSubscription = undefined;
        if (this.diffWatchStop) {
            await this.diffWatchStop();
            this.diffWatchStop = undefined;
        }
    }

    scrollToFile(path: string): void {
        const index = this.rowIndexByPath.get(path);
        if (index === undefined) {
            return;
        }
        this.diffViewport?.scrollToIndex(index, "smooth");
    }

    async openFileInVsCode(filePath: string, event?: Event): Promise<void> {
        event?.stopPropagation();
        const resolved = this.resolveFilePath(filePath);
        if (!resolved) {
            return;
        }
        try {
            await this.launcher.openFileInVsCode(resolved, 1, 1);
        } catch (error) {
            console.error("Failed to open file in VS Code", error);
        }
    }

    trackByRow(index: number): number {
        return index;
    }

    private buildRenderedRows(payload: DiffPayload): RenderedDiffRow[] {
        const highlightEnabled = true;
        const rows: RenderedDiffRow[] = [];
        const indexByPath = new Map<string, number>();
        let firstFile = true;
        for (const file of payload.files) {
            if (!firstFile) {
                rows.push({
                    kind: "spacer",
                    filePath: file.path,
                });
            }
            firstFile = false;
            if (!indexByPath.has(file.path)) {
                indexByPath.set(file.path, rows.length);
            }
            rows.push({
                kind: "header",
                filePath: file.path,
                status: file.status,
            });
            for (const line of file.lines) {
                if (line.type === "meta") {
                    continue;
                }
                rows.push({
                    kind: "line",
                    filePath: file.path,
                    line: {
                        type: line.type,
                        html: this.renderLine(
                            line.content,
                            file.path,
                            line.type,
                            highlightEnabled,
                        ),
                    },
                });
            }
        }
        this.rowIndexByPath = indexByPath;
        return rows;
    }

    private resolveFilePath(filePath: string): string | null {
        if (!this.worktreePath) {
            return null;
        }
        const base = this.worktreePath.replace(/[\\/]+$/, "");
        const separator = base.includes("\\") ? "\\" : "/";
        const normalized =
            separator === "\\" ? filePath.replace(/\//g, "\\") : filePath;
        return `${base}${separator}${normalized}`;
    }


    private buildFileTree(files: DiffPayload["files"]): FileTreeNode[] {
        type BuildNode = {
            name: string;
            path: string;
            isFile: boolean;
            status?: string;
            children: Map<string, BuildNode>;
        };
        const root: BuildNode = {
            name: "",
            path: "",
            isFile: false,
            children: new Map(),
        };
        for (const file of files) {
            const parts = file.path.split("/").filter(Boolean);
            let current = root;
            let currentPath = "";
            for (let index = 0; index < parts.length; index += 1) {
                const part = parts[index];
                currentPath = currentPath ? `${currentPath}/${part}` : part;
                let child = current.children.get(part);
                if (!child) {
                    child = {
                        name: part,
                        path: currentPath,
                        isFile: false,
                        children: new Map(),
                    };
                    current.children.set(part, child);
                }
                if (index === parts.length - 1) {
                    child.isFile = true;
                    child.status = file.status;
                }
                current = child;
            }
        }
        const compressNode = (node: BuildNode): BuildNode => {
            if (node.isFile) {
                return node;
            }
            const children = Array.from(node.children.values()).map(
                compressNode,
            );
            node.children = new Map(
                children.map((child) => [child.name, child]),
            );
            let current = node;
            while (!current.isFile && current.children.size === 1) {
                const onlyChild = Array.from(current.children.values())[0];
                if (onlyChild.isFile) {
                    break;
                }
                current = {
                    name: current.name
                        ? `${current.name}/${onlyChild.name}`
                        : onlyChild.name,
                    path: onlyChild.path,
                    isFile: false,
                    children: onlyChild.children,
                };
            }
            return current;
        };

        const toArray = (node: BuildNode, depth: number): FileTreeNode[] => {
            const children = Array.from(node.children.values()).sort((a, b) => {
                if (a.isFile !== b.isFile) {
                    return a.isFile ? 1 : -1;
                }
                return a.name.localeCompare(b.name);
            });
            return children.map((child) => ({
                name: child.name,
                path: child.path,
                depth,
                isFile: child.isFile,
                status: child.status,
                children: toArray(child, depth + 1),
            }));
        };
        const compressedRoot = compressNode(root);
        return toArray(compressedRoot, 0);
    }

    private renderLine(
        line: string,
        filePath: string,
        type: DiffLineType,
        highlightEnabled: boolean,
    ): SafeHtml {
        if (type === "add" || type === "del" || type === "context") {
            const highlighted = highlightEnabled
                ? this.highlightContent(line, filePath)
                : this.escapeHtml(line);
            return this.sanitizer.bypassSecurityTrustHtml(
                `<span class="diff-code">${highlighted}</span>`,
            );
        }
        return this.sanitizer.bypassSecurityTrustHtml(
            `<span class="diff-code">${this.escapeHtml(line)}</span>`,
        );
    }

    private highlightContent(content: string, filePath: string): string {
        const language = this.detectLanguage(filePath);
        if (language && hljs.getLanguage(language)) {
            try {
                return hljs.highlight(content, { language }).value;
            } catch {
                return this.escapeHtml(content);
            }
        }
        return this.escapeHtml(content);
    }

    private detectLanguage(path: string): string | null {
        const ext = path.split(".").pop()?.toLowerCase();
        if (!ext) {
            return null;
        }
        const mapping: Record<string, string> = {
            ts: "typescript",
            tsx: "typescript",
            js: "javascript",
            jsx: "javascript",
            json: "json",
            html: "html",
            htm: "html",
            css: "css",
            scss: "scss",
            sass: "scss",
            md: "markdown",
            xml: "xml",
            yml: "yaml",
            yaml: "yaml",
            sh: "bash",
            bash: "bash",
            py: "python",
            java: "java",
            go: "go",
            rs: "rust",
            cs: "csharp",
            c: "c",
            h: "c",
            cpp: "cpp",
            hpp: "cpp",
        };
        return mapping[ext] ?? null;
    }

    private escapeHtml(value: string): string {
        return value
            .replace(/&/g, "&amp;")
            .replace(/</g, "&lt;")
            .replace(/>/g, "&gt;");
    }
}
