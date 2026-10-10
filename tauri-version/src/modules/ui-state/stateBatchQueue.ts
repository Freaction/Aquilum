import { saveStateBatch } from './gateway';
import { viewKey } from './types';
import type { GraphCameraState, SaveStateBatchInput, ViewState } from './types';

export class StateBatchQueue {
  private sequence = 0;
  private pendingSession: SaveStateBatchInput['session'] = null;
  private pendingViews = new Map<string, ViewState>();
  private pendingGraphCamera: GraphCameraState | null = null;
  private draining = false;
  private disabled = false;
  private failures = 0;
  private retryTimer: ReturnType<typeof setTimeout> | null = null;

  constructor(
    private readonly workspaceId: string,
    private readonly windowId: string,
    private readonly epoch: string,
  ) {}

  queueSession(session: NonNullable<SaveStateBatchInput['session']>): void {
    if (this.disabled) return;
    this.pendingSession = {
      activeTabId: session.activeTabId,
      tabs: session.tabs ?? this.pendingSession?.tabs ?? null,
    };
    void this.drain();
  }

  queueView(view: ViewState): void {
    if (this.disabled) return;
    this.pendingViews.set(viewKey(view.documentId, view.paneId), view);
    void this.drain();
  }

  queueGraphCamera(camera: GraphCameraState): void {
    if (this.disabled) return;
    this.pendingGraphCamera = camera;
    void this.drain();
  }

  dispose(): void {
    this.disabled = true;
    this.pendingSession = null;
    this.pendingViews.clear();
    this.pendingGraphCamera = null;
    if (this.retryTimer) clearTimeout(this.retryTimer);
    this.retryTimer = null;
  }

  private async drain(): Promise<void> {
    if (this.draining || this.disabled) return;
    this.draining = true;
    let retry = false;
    try {
      while (this.hasPending()) {
        const session = this.pendingSession;
        const views = Array.from(this.pendingViews.values());
        const graphCamera = this.pendingGraphCamera;
        this.pendingSession = null;
        this.pendingViews.clear();
        this.pendingGraphCamera = null;
        try {
          const accepted = await saveStateBatch({
            workspaceId: this.workspaceId,
            windowId: this.windowId,
            epoch: this.epoch,
            sequence: ++this.sequence,
            nowMs: Date.now(),
            session,
            views,
            graphCamera,
          });
          if (!accepted) {
            this.dispose();
            break;
          }
          this.failures = 0;
        } catch (error) {
          console.error('Failed to save workspace session', error);
          if (this.disabled) break;
          this.requeue(session, views, graphCamera);
          this.failures += 1;
          retry = this.failures <= 3;
          break;
        }
      }
    } finally {
      this.draining = false;
      if (this.hasPending() && retry && !this.disabled) {
        this.retryTimer = setTimeout(() => {
          this.retryTimer = null;
          void this.drain();
        }, 500);
      } else if (this.hasPending() && this.failures === 0) {
        void this.drain();
      }
    }
  }

  private requeue(
    session: SaveStateBatchInput['session'],
    views: ViewState[],
    graphCamera: GraphCameraState | null,
  ): void {
    if (!this.pendingGraphCamera) this.pendingGraphCamera = graphCamera;
    const newer = this.pendingSession;
    if (!newer) this.pendingSession = session;
    else if (!newer.tabs && session?.tabs) {
      this.pendingSession = { ...newer, tabs: session.tabs };
    }
    for (const view of views) {
      const key = viewKey(view.documentId, view.paneId);
      if (!this.pendingViews.has(key)) this.pendingViews.set(key, view);
    }
  }

  private hasPending(): boolean {
    return this.pendingSession !== null
      || this.pendingViews.size > 0
      || this.pendingGraphCamera !== null;
  }
}
