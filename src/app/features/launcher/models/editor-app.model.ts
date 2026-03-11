export type EditorIcon =
    | "vscode"
    | "vscodeInsiders"
    | "vscodium"
    | "cursor"
    | "windsurf"
    | "zed"
    | "sublimeText"
    | "notepadPlusPlus"
    | "jetbrains"
    | "generic";

export interface EditorApp {
    id: string;
    name: string;
    icon: EditorIcon;
    iconDataUrl?: string | null;
}
