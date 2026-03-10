interface ImportMetaHot {
    accept(): void;
    dispose(cb: () => void): void;
}

interface ImportMeta {
    readonly hot?: ImportMetaHot;
}
