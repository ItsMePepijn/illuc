import { NgZone } from "@angular/core";

type ActivationRequest = {
    isActive: () => boolean;
    requestBottomSync: () => void;
};

export class CodexGuiBottomPinController {
    private pinnedToBottom = true;

    constructor(private readonly thresholdPx: number) {}

    get isPinned(): boolean {
        return this.pinnedToBottom;
    }

    pin(): void {
        this.pinnedToBottom = true;
    }

    updateFromScrollHost(scrollHost: HTMLElement): void {
        const distanceFromBottom =
            scrollHost.scrollHeight -
            scrollHost.scrollTop -
            scrollHost.clientHeight;
        this.pinnedToBottom = distanceFromBottom <= this.thresholdPx;
    }
}

export class CodexGuiBottomFollowController {
    private frameId?: number;

    constructor(
        private readonly zone: NgZone,
        private readonly onFollowApplied: () => void,
    ) {}

    request(scrollHost: HTMLElement | undefined, fallback: () => void): void {
        if (!scrollHost) {
            fallback();
            return;
        }

        this.clear();
        this.zone.runOutsideAngular(() => {
            this.frameId = requestAnimationFrame(() => {
                scrollHost.scrollTop = scrollHost.scrollHeight;
                this.onFollowApplied();
                this.frameId = undefined;
            });
        });
    }

    clear(): void {
        if (this.frameId === undefined) {
            return;
        }
        cancelAnimationFrame(this.frameId);
        this.frameId = undefined;
    }
}

export class CodexGuiBottomSyncController {
    private pending = false;
    private remainingAttempts = 0;
    private stableFrames = 0;
    private lastScrollHeight = 0;
    private scrollFrameId?: number;
    private activationTimeoutIds: number[] = [];

    constructor(
        private readonly zone: NgZone,
        private readonly maxAttempts: number,
        private readonly activationSyncDelaysMs: readonly number[],
        private readonly onSyncApplied: () => void,
    ) {}

    request(params: {
        scrollHost?: HTMLElement;
    }): void {
        this.pending = true;
        this.remainingAttempts = Math.max(
            this.remainingAttempts,
            this.maxAttempts,
        );
        this.stableFrames = 0;
        this.lastScrollHeight = 0;
        this.flush(params);
    }

    flush(params: {
        scrollHost?: HTMLElement;
    }): void {
        if (!this.pending) {
            return;
        }

        const { scrollHost } = params;
        if (!scrollHost) {
            return;
        }

        this.cancelScrollFrame();

        this.zone.runOutsideAngular(() => {
            this.scrollFrameId = requestAnimationFrame(() => {
                scrollHost.scrollTop = scrollHost.scrollHeight;
                this.remainingAttempts -= 1;
                this.onSyncApplied();

                const scrollHeight = scrollHost.scrollHeight;
                const distanceFromBottom =
                    scrollHeight -
                    scrollHost.scrollTop -
                    scrollHost.clientHeight;

                if (scrollHeight === this.lastScrollHeight) {
                    this.stableFrames += 1;
                } else {
                    this.stableFrames = 0;
                    this.lastScrollHeight = scrollHeight;
                }

                this.pending =
                    (distanceFromBottom > 1 || this.stableFrames < 2) &&
                    this.remainingAttempts > 0;
                this.scrollFrameId = undefined;

                if (this.pending) {
                    this.flush(params);
                }
            });
        });
    }

    scheduleActivationSync(request: ActivationRequest): void {
        this.clearActivationSync();
        this.zone.runOutsideAngular(() => {
            this.activationTimeoutIds = this.activationSyncDelaysMs.map(
                (delayMs) =>
                    window.setTimeout(() => {
                        if (!request.isActive()) {
                            return;
                        }
                        request.requestBottomSync();
                    }, delayMs),
            );
        });
    }

    clearActivationSync(): void {
        for (const timeoutId of this.activationTimeoutIds) {
            clearTimeout(timeoutId);
        }
        this.activationTimeoutIds = [];
    }

    clear(): void {
        this.pending = false;
        this.remainingAttempts = 0;
        this.stableFrames = 0;
        this.lastScrollHeight = 0;
        this.cancelScrollFrame();
    }

    private cancelScrollFrame(): void {
        if (this.scrollFrameId === undefined) {
            return;
        }
        cancelAnimationFrame(this.scrollFrameId);
        this.scrollFrameId = undefined;
    }
}
