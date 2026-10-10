import type { ViewUpdate } from '@codemirror/view';
import { EditorView } from '@codemirror/view';
import type { ViewState } from '../../../modules/ui-state';
import { captureScroll } from './scroll';
import { markOpenStage } from '../../../modules/perf/openTrace';
import type { ResolvedView } from './positions';
import { clamp } from '../../../modules/math';

interface ControllerConfig {
  documentId: string;
  path: () => string;
  initial: ViewState | null;
  resolved: ResolvedView | null;
  onChange: (state: ViewState) => void;
  revealOffset?: number;
}

const DEBOUNCE_MS = 100;
const MAX_WAIT_MS = 500;

export class ViewStateController {
  private view: EditorView | null = null;
  private scrollElement: HTMLElement | null = null;
  private debounce: ReturnType<typeof setTimeout> | null = null;
  private maxWait: ReturnType<typeof setTimeout> | null = null;
  private lastState: ViewState | null = null;
  private dirty = false;
  private restoring = false;
  private readonly onScroll = () => this.markDirty();
  private readonly onFocusOut = () => this.flush();
  private readonly onPageHide = () => this.flush();
  private readonly onVisibilityChange = () => {
    if (document.visibilityState !== 'hidden') return;
    this.flush();
  };

  constructor(private readonly config: ControllerConfig) {}

  attach(view: EditorView, scrollElement: HTMLElement): void {
    this.view = view;
    this.scrollElement = scrollElement;
    scrollElement.addEventListener('scroll', this.onScroll, { passive: true });
    view.contentDOM.addEventListener('focusout', this.onFocusOut);
    this.bindPageLifecycle();
    this.restore();
  }

  update(update: ViewUpdate): void {
    if (update.selectionSet || update.docChanged) this.markDirty();
  }

  dispose(captureCurrent = false): void {
    if (captureCurrent) this.dirty = true;
    this.flush();
    this.scrollElement?.removeEventListener('scroll', this.onScroll);
    this.view?.contentDOM.removeEventListener('focusout', this.onFocusOut);
    this.unbindPageLifecycle();
    this.clearTimers();
    this.settleRestore();
    this.view = null;
    this.scrollElement = null;
  }

  private liveTarget(): { view: EditorView; scrollElement: HTMLElement } | null {
    const view = this.view;
    const scrollElement = this.scrollElement;
    if (!view || !scrollElement) return null;
    if (!scrollElement.isConnected || !view.scrollDOM.isConnected) return null;
    if (scrollElement.checkVisibility?.({ visibilityProperty: true }) === false) return null;
    return { view, scrollElement };
  }

  private bindPageLifecycle(): void {
    window.addEventListener('pagehide', this.onPageHide);
    document.addEventListener('visibilitychange', this.onVisibilityChange);
  }

  private unbindPageLifecycle(): void {
    window.removeEventListener('pagehide', this.onPageHide);
    document.removeEventListener('visibilitychange', this.onVisibilityChange);
  }

  private restore(): void {
    const { initial, resolved, revealOffset } = this.config;
    const view = this.view;
    if (!view) return;
    if (revealOffset !== undefined) {
      const position = clamp(revealOffset, 0, view.state.doc.length);
      view.dispatch({
        selection: { anchor: position },
        effects: EditorView.scrollIntoView(position, { y: 'center' }),
      });
      view.focus();
      markOpenStage('scroll');
      return;
    }
    if (!initial || !resolved) {
      view.focus();
      markOpenStage('scroll');
      return;
    }
    view.focus();
    this.applyScroll(view, resolved.scroll, initial.scrollOffsetPx);
  }

  restoreNow(): void {
    const view = this.view;
    if (!view) return;
    const snapshot: Pick<ViewState, 'fallbackScrollAnchor' | 'scrollOffsetPx'> | null
      = this.lastState ?? this.config.initial;
    if (!snapshot) return;
    this.applyScroll(view, snapshot.fallbackScrollAnchor, snapshot.scrollOffsetPx);
  }

  private applyScroll(view: EditorView, anchor: number, offsetPx: number): void {
    this.restoring = true;
    view.dispatch({
      effects: EditorView.scrollIntoView(clamp(anchor, 0, view.state.doc.length), {
        y: 'start',
        yMargin: -offsetPx,
      }),
    });
    view.requestMeasure({
      read: () => null,
      write: () => {
        this.settleRestore();
        markOpenStage('scroll');
      },
    });
  }

  private settleRestore(): void {
    this.restoring = false;
  }

  private markDirty(): void {
    if (this.restoring || !this.view || !this.scrollElement) return;
    this.dirty = true;
    this.schedule();
  }

  private schedule(): void {
    if (this.debounce) clearTimeout(this.debounce);
    this.debounce = setTimeout(() => this.flush(), DEBOUNCE_MS);
    if (!this.maxWait) this.maxWait = setTimeout(() => this.flush(), MAX_WAIT_MS);
  }

  private flush(): void {
    if (this.restoring || !this.dirty) return;
    this.clearTimers();
    const target = this.liveTarget();
    if (target) this.lastState = this.capture(target.view, target.scrollElement);
    const state = this.lastState;
    if (!state) return;
    this.dirty = false;
    this.config.onChange(state);
  }

  private capture(view: EditorView, scrollElement: HTMLElement): ViewState {
    const selection = view.state.selection.main;
    const scroll = captureScroll(view, scrollElement);
    return {
      path: this.config.path(),
      documentId: this.config.documentId,
      paneId: 'main',
      cursorAnchor: [],
      cursorHead: [],
      fallbackAnchor: selection.anchor,
      fallbackHead: selection.head,
      scrollAnchor: [],
      fallbackScrollAnchor: scroll.anchor,
      scrollOffsetPx: scroll.offset,
      focusedSurface: 'body',
    };
  }

  private clearTimers(): void {
    if (this.debounce) clearTimeout(this.debounce);
    if (this.maxWait) clearTimeout(this.maxWait);
    this.debounce = null;
    this.maxWait = null;
  }
}
