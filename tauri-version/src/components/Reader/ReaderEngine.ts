import { Overlayer } from 'foliate-js/overlayer.js';
import { readerColumnWidthPx, readerPalette, readerStyleCss } from './readerStyle';
import type { ReaderSettings } from '../../modules/settings';
import { clamp } from '../../modules/math';

type ReaderTarget = string | number | { fraction: number };

type ReaderLocation = {
  cfi?: string;
  fraction?: number;
};

type ReaderOpenOptions = {
  lastLocation?: ReaderTarget;
};

type ReaderSelection = {
  text: string;
  cfi: string;
};

type ReaderQuoteMark = {
  cfi: string;
  label: string;
};

const SVG_NS = 'http://www.w3.org/2000/svg';
const QUOTE_MARK_SCALE = 0.6;

type QuoteMarkFont = {
  family: string;
  size: number;
  weight: string;
  style: string;
};

type QuoteMarkOptions = {
  color: string;
  label: string;
  font: QuoteMarkFont;
  blendMode: string;
  onActivate: () => void;
};

type DrawAnnotationDetail = {
  draw?: (
    draw: (rects: DOMRectList, options: QuoteMarkOptions) => SVGElement,
    options: QuoteMarkOptions,
  ) => void;
  annotation?: { value?: string };
  range?: Range;
};

function quoteMarkFont(range?: Range): QuoteMarkFont {
  const node = range?.endContainer;
  const element = node?.nodeType === Node.ELEMENT_NODE
    ? node as Element
    : node?.parentElement ?? null;
  const style = element?.ownerDocument.defaultView?.getComputedStyle(element);
  return {
    family: style?.fontFamily || 'serif',
    size: parseFloat(style?.fontSize ?? '') || 16,
    weight: style?.fontWeight || 'normal',
    style: style?.fontStyle || 'normal',
  };
}

function drawQuoteMark(rects: DOMRectList, options: QuoteMarkOptions): SVGElement {
  const group = document.createElementNS(SVG_NS, 'g');
  const highlight = Overlayer.highlight(rects, { color: options.color });
  highlight.style.mixBlendMode = options.blendMode;
  group.append(highlight);

  const last = rects[rects.length - 1];
  if (!last) return group;

  const { font } = options;
  const markSize = font.size * QUOTE_MARK_SCALE;
  const glyphTop = last.top + (last.height - font.size) / 2;
  const mark = document.createElementNS(SVG_NS, 'text');
  mark.textContent = options.label;
  mark.setAttribute('x', String(last.right + font.size * 0.15));
  mark.setAttribute('y', String(glyphTop + markSize * 0.85));
  mark.setAttribute('fill', options.color);
  mark.style.fontFamily = font.family;
  mark.style.fontSize = `${markSize}px`;
  mark.style.fontWeight = font.weight;
  mark.style.fontStyle = font.style;
  mark.style.cursor = 'pointer';
  mark.style.pointerEvents = 'auto';
  mark.addEventListener('click', options.onActivate);
  group.append(mark);
  return group;
}

let foliateImport: Promise<unknown> | null = null;

export function prefetchFoliate(): Promise<unknown> {
  if (!foliateImport) foliateImport = import('foliate-js/view.js');
  return foliateImport;
}

type FoliateRenderer = HTMLElement & {
  start: number;
  setStyles?: (styles: string) => void;
  scrollBy: (dx: number, dy: number) => void;
  next: (distance?: number) => Promise<void>;
  prev: (distance?: number) => Promise<void>;
};

type FoliateView = HTMLElement & {
  open: (book: File | Blob | string) => Promise<void>;
  close: () => void;
  init: (opts: { lastLocation?: unknown; showTextStart?: boolean }) => Promise<void>;
  next: (distance?: number) => Promise<void>;
  prev: (distance?: number) => Promise<void>;
  getCFI: (index: number, range?: Range) => string;
  addAnnotation: (annotation: { value: string }, remove?: boolean) => Promise<void>;
  renderer?: FoliateRenderer;
  book?: { destroy?: () => void };
  lastLocation?: {
    cfi?: string;
    fraction?: number;
  };
};

const SELECTION_SETTLE_MS = 120;
const EDGE_ZONE_PX = 56;
const EDGE_DWELL_MS = 650;
const PAGE_INSET_PX = 6;

type CaretPoint = { node: Node; offset: number };

function caretPointAt(doc: Document, x: number, y: number): CaretPoint | null {
  const legacy = doc as Document & {
    caretRangeFromPoint?: (x: number, y: number) => Range | null;
  };
  const range = legacy.caretRangeFromPoint?.(x, y);
  if (range) return { node: range.startContainer, offset: range.startOffset };
  const standard = doc as Document & {
    caretPositionFromPoint?: (x: number, y: number) => { offsetNode: Node; offset: number } | null;
  };
  const position = standard.caretPositionFromPoint?.(x, y);
  return position ? { node: position.offsetNode, offset: position.offset } : null;
}

type Debounced = (() => void) & { cancel: () => void };

function debounce(fn: () => void, ms: number): Debounced {
  let timer: ReturnType<typeof setTimeout> | null = null;
  const run = () => {
    if (timer !== null) clearTimeout(timer);
    timer = setTimeout(() => {
      timer = null;
      fn();
    }, ms);
  };
  run.cancel = () => {
    if (timer !== null) clearTimeout(timer);
    timer = null;
  };
  return run;
}

export class FoliateReaderEngine {
  private view: FoliateView | null = null;
  private docIndex = new WeakMap<Document, number>();
  private relocateListeners = new Set<(location: ReaderLocation) => void>();
  private selectionListeners = new Set<(selection: ReaderSelection | null) => void>();
  private quoteMarks = new Map<string, string>();
  private quoteRefListeners = new Set<(cfi: string) => void>();
  private docCleanups = new Map<Document, () => void>();
  private selecting = false;
  private pinnedStart: number | null = null;
  private lastSelectionKey: string | null = null;
  private pageForward: (() => Promise<void>) | null = null;
  private pageBack: (() => Promise<void>) | null = null;
  private edgeDirection: -1 | 0 | 1 = 0;
  private edgeTimer: ReturnType<typeof setInterval> | null = null;
  private lastPointer: { x: number; y: number } | null = null;
  private desiredCaret: CaretPoint | null = null;
  private caretFrame: number | null = null;
  private settings: ReaderSettings | null = null;
  private appliedSettingsKey: string | null = null;

  private onQuoteDrawAnnotation = (event: Event) => {
    const detail = (event as CustomEvent).detail as DrawAnnotationDetail;
    const value = detail.annotation?.value;
    if (!value) return;
    const label = this.quoteMarks.get(value);
    if (label === undefined) return;
    const accent = getComputedStyle(document.documentElement)
      .getPropertyValue('--q-text-accent').trim() || 'currentColor';
    detail.draw?.(drawQuoteMark, {
      color: accent,
      label,
      font: quoteMarkFont(detail.range),
      blendMode: document.documentElement.getAttribute('data-theme') === 'dark'
        ? 'lighten'
        : 'multiply',
      onActivate: () => this.emitQuoteRef(value),
    });
  };

  private onCreateOverlay = () => {
    void this.redrawQuoteMarks();
  };

  private onShowAnnotation = (event: Event) => {
    const detail = (event as CustomEvent).detail as { value?: string; range?: Range };
    const value = detail?.value;
    if (!value || !this.quoteMarks.has(value)) return;
    const sel = detail.range?.startContainer.ownerDocument?.getSelection();
    if (sel && !sel.isCollapsed) return;
    this.emitQuoteRef(value);
  };

  private emitQuoteRef(cfi: string): void {
    for (const listener of this.quoteRefListeners) listener(cfi);
  }

  private async redrawQuoteMarks(): Promise<void> {
    const view = this.view;
    if (!view) return;
    for (const cfi of this.quoteMarks.keys()) {
      try {
        await view.addAnnotation({ value: cfi });
      } catch (error) {
        console.warn('Failed to draw a quote mark', cfi, error);
      }
    }
  }

  async setQuotes(marks: ReaderQuoteMark[]): Promise<void> {
    const previous = this.quoteMarks;
    const next = new Map(marks.map((mark) => [mark.cfi, mark.label]));
    this.quoteMarks = next;

    const view = this.view;
    if (!view) return;

    for (const cfi of previous.keys()) {
      if (next.has(cfi)) continue;
      try {
        await view.addAnnotation({ value: cfi }, true);
      } catch (error) {
        console.warn('Failed to remove a quote mark', cfi, error);
      }
    }
    for (const [cfi, label] of next) {
      if (previous.get(cfi) === label) continue;
      try {
        await view.addAnnotation({ value: cfi });
      } catch (error) {
        console.warn('Failed to draw a quote mark', cfi, error);
      }
    }
  }

  onQuoteRef(listener: (cfi: string) => void): () => void {
    this.quoteRefListeners.add(listener);
    return () => {
      this.quoteRefListeners.delete(listener);
    };
  }

  private onViewRelocate = (event: Event) => {
    const detail = (event as CustomEvent).detail as FoliateView['lastLocation'];
    if (!detail) return;
    const location: ReaderLocation = {
      cfi: detail.cfi,
      fraction: detail.fraction,
    };
    for (const listener of this.relocateListeners) listener(location);
  };

  private onViewLoad = (event: Event) => {
    const detail = (event as CustomEvent).detail as { doc?: Document; index?: number };
    if (!detail?.doc) return;
    const doc = detail.doc;
    if (typeof detail.index === 'number') this.docIndex.set(doc, detail.index);
    this.detachAllSelectionTracking();
    this.attachSelectionTracking(doc);
  };

  applyStyle(settings: ReaderSettings): void {
    this.settings = settings;
    const renderer = this.renderer;
    if (!renderer) return;
    const key = JSON.stringify(settings);
    if (key === this.appliedSettingsKey) return;
    this.appliedSettingsKey = key;
    renderer.setStyles?.(readerStyleCss(settings, readerPalette()));
    renderer.setAttribute('flow', settings.flow);
    renderer.setAttribute('margin', `${settings.marginPx}px`);
    void readerColumnWidthPx(settings).then((width) => {
      if (this.appliedSettingsKey !== key) return;
      this.renderer?.setAttribute('max-inline-size', `${width}px`);
    });
  }

  private get renderer(): FoliateRenderer | null {
    return this.view?.renderer ?? null;
  }

  private onRendererScroll = () => {
    if (!this.selecting || this.pinnedStart === null) return;
    const renderer = this.renderer;
    if (!renderer) return;
    const drift = renderer.start - this.pinnedStart;
    if (Math.abs(drift) < 1) return;
    renderer.scrollBy(-drift, 0);
  };

  private onRendererRelocate = () => {
    if (!this.selecting) return;
    this.pinnedStart = this.renderer?.start ?? null;
  };

  private readSelectionFrom(doc: Document): ReaderSelection | null {
    const view = this.view;
    if (!view) return null;
    const sel = doc.getSelection?.() ?? doc.defaultView?.getSelection();
    if (!sel || sel.isCollapsed || sel.rangeCount === 0) return null;
    const range = sel.getRangeAt(0);
    const text = range.toString().replace(/\s+/g, ' ').trim();
    if (!text) return null;
    const index = this.docIndex.get(doc) ?? 0;
    try {
      const cfi = view.getCFI(index, range);
      return { text, cfi };
    } catch {
      const fallback = view.lastLocation?.cfi;
      return fallback ? { text, cfi: fallback } : { text, cfi: '' };
    }
  }

  private emitSelection(next: ReaderSelection | null): void {
    const key = next ? `${next.cfi}\n${next.text}` : null;
    if (key === this.lastSelectionKey) return;
    this.lastSelectionKey = key;
    for (const listener of this.selectionListeners) listener(next);
  }

  private visiblePageBox(doc: Document): DOMRect {
    const frame = doc.defaultView?.frameElement as HTMLElement | null;
    const page = frame?.closest<HTMLElement>('#container') ?? this.host;
    return page.getBoundingClientRect();
  }

  private updateDesiredCaret(doc: Document, clientX: number, clientY: number): void {
    const frame = doc.defaultView?.frameElement as HTMLElement | null;
    if (!frame) return;
    const page = this.visiblePageBox(doc);
    const x = clamp(clientX, page.left + PAGE_INSET_PX, page.right - PAGE_INSET_PX);
    const y = clamp(clientY, page.top + PAGE_INSET_PX, page.bottom - PAGE_INSET_PX);
    const origin = frame.getBoundingClientRect();
    const caret = caretPointAt(doc, x - origin.left, y - origin.top);
    if (caret) this.desiredCaret = caret;
  }

  private enforceDesiredCaret(doc: Document): void {
    const caret = this.desiredCaret;
    if (!caret) return;
    const sel = doc.getSelection();
    if (!sel || sel.rangeCount === 0 || !sel.anchorNode) return;
    if (sel.focusNode === caret.node && sel.focusOffset === caret.offset) return;
    try {
      sel.extend(caret.node, caret.offset);
    } catch {}
  }

  private scheduleCaretEnforcement(doc: Document): void {
    if (this.caretFrame !== null) return;
    this.caretFrame = requestAnimationFrame(() => {
      this.caretFrame = null;
      if (this.selecting) this.enforceDesiredCaret(doc);
    });
  }

  private stopEdgeTurning(): void {
    if (this.edgeTimer !== null) clearInterval(this.edgeTimer);
    this.edgeTimer = null;
    this.edgeDirection = 0;
  }

  private updateEdgeTurning(doc: Document, clientX: number): void {
    const page = this.visiblePageBox(doc);
    const zone = Math.min(EDGE_ZONE_PX, page.width / 6);
    const direction = clientX >= page.right - zone ? 1
      : clientX <= page.left + zone ? -1
      : 0;
    if (direction === this.edgeDirection) return;
    this.stopEdgeTurning();
    if (direction === 0) return;
    this.edgeDirection = direction;
    this.edgeTimer = setInterval(() => this.turnPageWhileSelecting(doc, direction), EDGE_DWELL_MS);
  }

  private turnPageWhileSelecting(doc: Document, direction: -1 | 1): void {
    const turn = direction > 0 ? this.pageForward : this.pageBack;
    if (!turn) return;
    void turn().then(() => {
      const pointer = this.lastPointer;
      if (!this.selecting || !pointer) return;
      this.updateDesiredCaret(doc, pointer.x, pointer.y);
      this.enforceDesiredCaret(doc);
    });
  }

  private attachSelectionTracking(doc: Document): void {
    this.detachSelectionTracking(doc);

    const commit = () => this.emitSelection(this.readSelectionFrom(doc));
    const commitWhenSettled = debounce(commit, SELECTION_SETTLE_MS);

    const startSelecting = (event: PointerEvent) => {
      if (event.button !== 0) return;
      this.selecting = true;
      this.pinnedStart = this.renderer?.start ?? null;
      this.lastPointer = null;
      this.desiredCaret = null;
      commitWhenSettled.cancel();
      this.emitSelection(null);
    };

    const stopSelecting = () => {
      if (!this.selecting) return;
      this.selecting = false;
      this.pinnedStart = null;
      this.lastPointer = null;
      this.desiredCaret = null;
      this.stopEdgeTurning();
      commit();
    };

    const trackPointer = (clientX: number, clientY: number) => {
      if (!this.selecting) return;
      this.lastPointer = { x: clientX, y: clientY };
      this.updateDesiredCaret(doc, clientX, clientY);
      this.scheduleCaretEnforcement(doc);
      this.updateEdgeTurning(doc, clientX);
    };

    const onDocPointerMove = (event: PointerEvent) => {
      const origin = (doc.defaultView?.frameElement as HTMLElement | null)?.getBoundingClientRect();
      if (!origin) return;
      trackPointer(event.clientX + origin.left, event.clientY + origin.top);
    };

    const onHostPointerMove = (event: PointerEvent) => {
      trackPointer(event.clientX, event.clientY);
    };

    const onSelectionChange = () => {
      if (this.selecting) {
        this.enforceDesiredCaret(doc);
        return;
      }
      commitWhenSettled();
    };

    doc.addEventListener('pointerdown', startSelecting);
    doc.addEventListener('pointermove', onDocPointerMove);
    doc.addEventListener('pointerup', stopSelecting);
    doc.addEventListener('pointercancel', stopSelecting);
    doc.addEventListener('keyup', commitWhenSettled);
    doc.addEventListener('selectionchange', onSelectionChange);
    window.addEventListener('pointermove', onHostPointerMove);
    window.addEventListener('pointerup', stopSelecting);
    window.addEventListener('pointercancel', stopSelecting);

    this.docCleanups.set(doc, () => {
      commitWhenSettled.cancel();
      doc.removeEventListener('pointerdown', startSelecting);
      doc.removeEventListener('pointermove', onDocPointerMove);
      doc.removeEventListener('pointerup', stopSelecting);
      doc.removeEventListener('pointercancel', stopSelecting);
      doc.removeEventListener('keyup', commitWhenSettled);
      doc.removeEventListener('selectionchange', onSelectionChange);
      window.removeEventListener('pointermove', onHostPointerMove);
      window.removeEventListener('pointerup', stopSelecting);
      window.removeEventListener('pointercancel', stopSelecting);
    });
  }

  private detachSelectionTracking(doc: Document): void {
    const cleanup = this.docCleanups.get(doc);
    if (!cleanup) return;
    cleanup();
    this.docCleanups.delete(doc);
  }

  private detachAllSelectionTracking(): void {
    for (const cleanup of this.docCleanups.values()) cleanup();
    this.docCleanups.clear();
  }

  constructor(private readonly host: HTMLElement) {}

  async open(file: File, options?: ReaderOpenOptions): Promise<void> {
    await prefetchFoliate();
    this.close();

    const view = document.createElement('foliate-view') as FoliateView;
    view.className = 'q-foliate-view';
    view.style.display = 'block';
    view.style.width = '100%';
    view.style.height = '100%';
    view.addEventListener('relocate', this.onViewRelocate);
    view.addEventListener('load', this.onViewLoad);
    view.addEventListener('draw-annotation', this.onQuoteDrawAnnotation);
    view.addEventListener('create-overlay', this.onCreateOverlay);
    view.addEventListener('show-annotation', this.onShowAnnotation);
    this.host.replaceChildren(view);
    this.view = view;

    await view.open(file);
    const renderer = view.renderer;
    if (renderer) {
      renderer.addEventListener('scroll', this.onRendererScroll);
      renderer.addEventListener('relocate', this.onRendererRelocate);
      const turnNext = renderer.next.bind(renderer);
      const turnPrev = renderer.prev.bind(renderer);
      this.pageForward = () => turnNext();
      this.pageBack = () => turnPrev();
      renderer.next = (distance) => (this.selecting ? Promise.resolve() : turnNext(distance));
      renderer.prev = (distance) => (this.selecting ? Promise.resolve() : turnPrev(distance));
      if (this.settings) this.applyStyle(this.settings);
    }
    if (options?.lastLocation != null) {
      await view.init({ lastLocation: options.lastLocation });
    } else {
      await view.init({ showTextStart: true });
    }
  }

  close(): void {
    this.detachAllSelectionTracking();
    this.stopEdgeTurning();
    if (this.caretFrame !== null) cancelAnimationFrame(this.caretFrame);
    this.caretFrame = null;
    this.selecting = false;
    this.pinnedStart = null;
    this.lastSelectionKey = null;
    this.lastPointer = null;
    this.desiredCaret = null;
    this.pageForward = null;
    this.pageBack = null;
    this.appliedSettingsKey = null;

    const view = this.view;
    if (!view) {
      this.host.replaceChildren();
      return;
    }
    view.renderer?.removeEventListener('scroll', this.onRendererScroll);
    view.renderer?.removeEventListener('relocate', this.onRendererRelocate);
    view.removeEventListener('relocate', this.onViewRelocate);
    view.removeEventListener('load', this.onViewLoad);
    view.removeEventListener('draw-annotation', this.onQuoteDrawAnnotation);
    view.removeEventListener('create-overlay', this.onCreateOverlay);
    view.removeEventListener('show-annotation', this.onShowAnnotation);
    try {
      view.close();
    } catch (error) {
      console.error('Failed to close the book view', error);
    }
    try {
      view.book?.destroy?.();
    } catch (error) {
      console.error('Failed to release the book', error);
    }
    view.remove();
    this.view = null;
    this.host.replaceChildren();
  }

  async next(): Promise<void> {
    await this.view?.next();
  }

  async prev(): Promise<void> {
    await this.view?.prev();
  }

  onRelocate(listener: (location: ReaderLocation) => void): () => void {
    this.relocateListeners.add(listener);
    return () => {
      this.relocateListeners.delete(listener);
    };
  }

  onSelection(listener: (selection: ReaderSelection | null) => void): () => void {
    this.selectionListeners.add(listener);
    return () => {
      this.selectionListeners.delete(listener);
    };
  }
}
