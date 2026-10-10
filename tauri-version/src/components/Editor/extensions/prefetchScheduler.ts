import type { EditorView } from '@codemirror/view';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { LINKS_CHANGED_EVENT, type IndexRevisionEvent } from '../../../modules/links/events';
import { samePath } from '../../../modules/paths';
import { livePreviewConfigFacet } from './livePreviewConfig';

export type PrefetchDelays = {
  editMs: number;
  revisionMs: number;
};

export class PrefetchScheduler {
  private debounce: ReturnType<typeof setTimeout> | null = null;
  private queue: Promise<void> = Promise.resolve();
  private pendingRefresh = false;
  private unlisten: UnlistenFn | null = null;
  private stopped = false;

  constructor(
    private readonly view: EditorView,
    private readonly delays: PrefetchDelays,
    private readonly work: (refresh: boolean) => Promise<void>,
  ) {
    void this.listenForRevisions().then(() => this.enqueue(() => this.work(false)));
  }

  get destroyed(): boolean {
    return this.stopped;
  }

  schedule(refresh: boolean): void {
    if (this.stopped) return;
    this.pendingRefresh ||= refresh;
    if (this.debounce !== null) clearTimeout(this.debounce);
    this.debounce = setTimeout(() => {
      this.debounce = null;
      const refreshNow = this.pendingRefresh;
      this.pendingRefresh = false;
      this.enqueue(() => this.work(refreshNow));
    }, refresh ? this.delays.revisionMs : this.delays.editMs);
  }

  enqueue(work: () => Promise<void>): void {
    if (this.stopped) return;
    this.queue = this.queue.then(work).catch((error) => {
      console.error('[prefetch] work failed', error);
    });
  }

  destroy(): void {
    this.stopped = true;
    if (this.debounce !== null) clearTimeout(this.debounce);
    this.debounce = null;
    this.unlisten?.();
    this.unlisten = null;
  }

  private async listenForRevisions(): Promise<void> {
    try {
      const dispose = await listen<IndexRevisionEvent>(LINKS_CHANGED_EVENT, (event) => {
        const workspacePath = this.view.state.facet(livePreviewConfigFacet)?.workspacePath;
        if (!samePath(event.payload.workspacePath, workspacePath)) return;
        this.schedule(true);
      });
      if (this.stopped) dispose();
      else this.unlisten = dispose;
    } catch (error) {
      console.error('[prefetch] revision listener failed', error);
    }
  }
}
