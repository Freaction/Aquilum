export class WidgetDocFlush {
    private dirty = false;
    private timer: ReturnType<typeof setTimeout> | null = null;

    constructor(
        private readonly onFlush: () => void,
        private readonly delayMs: number,
    ) {}

    get isDirty(): boolean {
        return this.dirty;
    }

    markDirty(): void {
        this.dirty = true;
        this.schedule();
    }

    clearDirty(): void {
        this.dirty = false;
    }

    flushNow(): void {
        this.cancelTimer();
        if (!this.dirty) return;
        this.dirty = false;
        this.onFlush();
    }

    cancel(): void {
        this.cancelTimer();
    }

    private schedule(): void {
        this.cancelTimer();
        this.timer = setTimeout(() => this.flushNow(), this.delayMs);
    }

    private cancelTimer(): void {
        if (this.timer != null) {
            clearTimeout(this.timer);
            this.timer = null;
        }
    }
}
