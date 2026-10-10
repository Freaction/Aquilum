import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { EditorView, ViewUpdate } from '@codemirror/view';
import type { ViewState } from '../../../modules/ui-state';
import { ViewStateController } from './controller';
import { fallbackView } from './positions';

function resolvedFor(initial: ViewState | null) {
  return initial ? fallbackView(initial) : null;
}

function stubPageLifecycle() {
  const windowListeners = new Map<string, EventListener>();
  const documentListeners = new Map<string, EventListener>();
  let visibilityState = 'visible';
  vi.stubGlobal('window', {
    addEventListener: (type: string, listener: EventListener) => {
      windowListeners.set(type, listener);
    },
    removeEventListener: (type: string) => {
      windowListeners.delete(type);
    },
    dispatchEvent: (event: Event) => {
      windowListeners.get(event.type)?.(event);
      return true;
    },
  });
  vi.stubGlobal('document', {
    get visibilityState() { return visibilityState; },
    set visibilityState(value: string) { visibilityState = value; },
    addEventListener: (type: string, listener: EventListener) => {
      documentListeners.set(type, listener);
    },
    removeEventListener: (type: string) => {
      documentListeners.delete(type);
    },
    dispatchEvent: (event: Event) => {
      documentListeners.get(event.type)?.(event);
      return true;
    },
  });
}

function element(top: number): HTMLElement & {
  emit(type: string): void;
  setTop(value: number): void;
  setReach(value: number): void;
  setVisible(value: boolean): void;
  isConnected: boolean;
} {
  const listeners = new Map<string, EventListener>();
  let currentTop = top;
  let connected = true;
  let visible = true;
  let reach = Number.POSITIVE_INFINITY;
  let scrollTop = 0;
  return {
    get scrollTop() { return scrollTop; },
    set scrollTop(value: number) { scrollTop = Math.min(Math.max(0, value), reach); },
    setReach: (value: number) => { reach = value; },
    get isConnected() { return connected; },
    set isConnected(value: boolean) { connected = value; },
    addEventListener: vi.fn((type: string, listener: EventListenerOrEventListenerObject) => {
      if (typeof listener === 'function') listeners.set(type, listener);
    }),
    removeEventListener: vi.fn((type: string) => listeners.delete(type)),
    getBoundingClientRect: () => ({ top: currentTop }),
    checkVisibility: () => visible,
    emit: (type: string) => listeners.get(type)?.(new Event(type)),
    setTop: (value: number) => { currentTop = value; },
    setVisible: (value: boolean) => { visible = value; },
  } as unknown as HTMLElement & {
    emit(type: string): void;
    setTop(value: number): void;
    setReach(value: number): void;
    setVisible(value: boolean): void;
    isConnected: boolean;
  };
}

function createView(scrollDOM: HTMLElement, contentDOM: HTMLElement = element(0)) {
  return {
    contentDOM,
    scrollDOM,
    focus: vi.fn(),
    state: { selection: { main: { anchor: 4, head: 4 } } },
    lineBlockAtHeight: () => ({ from: 2, top: 100 }),
    coordsAtPos: () => ({ top: 0 }),
  } as unknown as EditorView;
}

describe('ViewStateController tab lifecycle', () => {
  beforeEach(() => {
    stubPageLifecycle();
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('captures the current state synchronously when a tab unmounts', () => {
    const onChange = vi.fn();
    const scrollElement = element(0);
    scrollElement.scrollTop = 120;
    const scrollDOM = element(-100);
    const view = createView(scrollDOM);
    const controller = new ViewStateController({
      documentId: 'doc',
      path: () => 'note.md',
      initial: null,
      resolved: resolvedFor(null),
      onChange,
    });

    controller.attach(view, scrollElement);
    controller.update({ selectionSet: true, docChanged: false } as ViewUpdate);
    controller.dispose();

    expect(onChange).toHaveBeenCalledTimes(1);
    expect(onChange.mock.calls[0][0]).toMatchObject({
      path: 'note.md',
      documentId: 'doc',
      cursorAnchor: [],
      fallbackAnchor: 4,
      fallbackScrollAnchor: 2,
    });
  });

  it('does not replace a valid snapshot while the tab is live but hidden', () => {
    vi.useFakeTimers();
    const onChange = vi.fn();
    const scrollElement = element(0);
    scrollElement.scrollTop = 120;
    const scrollDOM = element(-100);
    let shown = true;
    const view = {
      contentDOM: element(0),
      scrollDOM,
      focus: vi.fn(),
      state: { selection: { main: { anchor: 4, head: 4 } } },
      lineBlockAtHeight: () => shown
        ? { from: 2, top: 100 }
        : { from: 0, top: 0 },
      coordsAtPos: () => ({ top: 0 }),
    } as unknown as EditorView;
    const controller = new ViewStateController({
      documentId: 'doc',
      path: () => 'note.md',
      initial: null,
      resolved: resolvedFor(null),
      onChange,
    });

    controller.attach(view, scrollElement);
    scrollElement.emit('scroll');
    vi.advanceTimersByTime(100);
    expect(onChange.mock.calls[0][0]).toMatchObject({ fallbackScrollAnchor: 2 });

    shown = false;
    scrollElement.setVisible(false);
    scrollElement.scrollTop = 0;
    scrollDOM.setTop(0);
    controller.update({ selectionSet: true, docChanged: false } as ViewUpdate);
    vi.advanceTimersByTime(100);

    for (const call of onChange.mock.calls) {
      expect(call[0]).toMatchObject({ fallbackScrollAnchor: 2 });
    }
    controller.dispose();
    vi.useRealTimers();
  });

  it('does not replace a valid scroll snapshot after CodeMirror detaches its DOM', () => {
    vi.useFakeTimers();
    const onChange = vi.fn();
    const scrollElement = element(0);
    scrollElement.scrollTop = 120;
    const scrollDOM = element(-100);
    let attached = true;
    const view = {
      contentDOM: element(0),
      scrollDOM,
      focus: vi.fn(),
      state: { selection: { main: { anchor: 4, head: 4 } } },
      lineBlockAtHeight: () => attached
        ? { from: 2, top: 100 }
        : { from: 0, top: 0 },
      coordsAtPos: () => ({ top: 0 }),
    } as unknown as EditorView;
    const controller = new ViewStateController({
      documentId: 'doc',
      path: () => 'note.md',
      initial: null,
      resolved: resolvedFor(null),
      onChange,
    });

    controller.attach(view, scrollElement);
    scrollElement.emit('scroll');
    vi.advanceTimersByTime(100);
    attached = false;
    scrollElement.scrollTop = 0;
    scrollDOM.setTop(0);
    scrollElement.isConnected = false;
    scrollDOM.isConnected = false;
    controller.dispose(true);

    for (const call of onChange.mock.calls) {
      expect(call[0]).toMatchObject({
        fallbackAnchor: 4,
        fallbackScrollAnchor: 2,
        scrollOffsetPx: 0,
      });
    }
    expect(onChange).toHaveBeenCalled();
    vi.useRealTimers();
  });

  it('captures layout-only scroll changes before the editor DOM is removed', () => {
    const onChange = vi.fn();
    const scrollElement = element(0);
    const scrollDOM = element(-100);
    const view = createView(scrollDOM);
    const controller = new ViewStateController({
      documentId: 'doc',
      path: () => 'note.md',
      initial: null,
      resolved: resolvedFor(null),
      onChange,
    });

    controller.attach(view, scrollElement);
    scrollElement.scrollTop = 120;
    controller.dispose(true);

    expect(onChange).toHaveBeenCalledTimes(1);
    expect(onChange.mock.calls[0][0]).toMatchObject({
      fallbackScrollAnchor: 2,
      scrollOffsetPx: 0,
    });
  });

  it('does not touch layout while the reader is scrolling', () => {
    const scrollElement = element(0);
    scrollElement.scrollTop = 120;
    const scrollDOM = element(-100);
    const view = createView(scrollDOM);
    const measured = vi.spyOn(view, 'coordsAtPos');
    const controller = new ViewStateController({
      documentId: 'doc',
      path: () => 'note.md',
      initial: null,
      resolved: resolvedFor(null),
      onChange: vi.fn(),
    });

    controller.attach(view, scrollElement);
    for (let tick = 0; tick < 30; tick += 1) {
      scrollElement.scrollTop = 120 + tick * 40;
      scrollElement.emit('scroll');
    }

    expect(measured).not.toHaveBeenCalled();

    controller.dispose();

    expect(measured).toHaveBeenCalledTimes(1);
  });

  it('flushes on pagehide without waiting for debounce', () => {
    const onChange = vi.fn();
    const scrollElement = element(0);
    scrollElement.scrollTop = 120;
    const scrollDOM = element(-100);
    const view = createView(scrollDOM);
    const controller = new ViewStateController({
      documentId: 'doc',
      path: () => 'note.md',
      initial: null,
      resolved: resolvedFor(null),
      onChange,
    });

    controller.attach(view, scrollElement);
    scrollElement.emit('scroll');
    expect(onChange).not.toHaveBeenCalled();
    window.dispatchEvent(new Event('pagehide'));

    expect(onChange).toHaveBeenCalledTimes(1);
    expect(onChange.mock.calls[0][0]).toMatchObject({
      fallbackScrollAnchor: 2,
    });

    controller.dispose();
  });

  it('flushes on visibility hidden', () => {
    const onChange = vi.fn();
    const scrollElement = element(0);
    scrollElement.scrollTop = 80;
    const scrollDOM = element(-100);
    const view = createView(scrollDOM);
    const controller = new ViewStateController({
      documentId: 'doc',
      path: () => 'note.md',
      initial: null,
      resolved: resolvedFor(null),
      onChange,
    });

    controller.attach(view, scrollElement);
    scrollElement.emit('scroll');
    (document as { visibilityState: string }).visibilityState = 'hidden';
    document.dispatchEvent(new Event('visibilitychange'));

    expect(onChange).toHaveBeenCalledTimes(1);
    controller.dispose();
  });
});

interface ScrollTargetSpec {
  range: { head: number };
  y: string;
  yMargin: number;
}

function restoringView(scrollElement: ReturnType<typeof element>) {
  const measures: Array<{ read: (view: unknown) => unknown; write: (value: unknown, view: unknown) => void }> = [];
  const scrollDOM = {
    isConnected: true,
    getBoundingClientRect: () => ({ top: -scrollElement.scrollTop }),
  } as unknown as HTMLElement;
  const targets: ScrollTargetSpec[] = [];
  const view = {
    contentDOM: element(0),
    scrollDOM,
    focus: vi.fn(),
    state: { selection: { main: { anchor: 4, head: 4 } }, doc: { length: 10 } },
    lineBlockAtHeight: (height: number) => ({ from: 2, top: height }),
    coordsAtPos: () => ({ top: 0 }),
    dispatch: (spec: { effects?: { value: ScrollTargetSpec } }) => {
      if (spec.effects) targets.push(spec.effects.value);
    },
    requestMeasure: (measure: { read: (view: unknown) => unknown; write: (value: unknown, view: unknown) => void }) => {
      measures.push(measure);
    },
  };
  return {
    view: view as unknown as EditorView,
    targets,
    frame(): boolean {
      const measure = measures.shift();
      if (!measure) return false;
      measure.write(measure.read(view), view);
      return true;
    },
    pending: () => measures.length,
  };
}

function storedTop(): {
  documentId: string;
  paneId: string;
  cursorAnchor: number[];
  cursorHead: number[];
  fallbackAnchor: number;
  fallbackHead: number;
  scrollAnchor: number[];
  fallbackScrollAnchor: number;
  scrollOffsetPx: number;
  focusedSurface: string;
} {
  return {
    documentId: 'doc',
    paneId: 'main',
    cursorAnchor: [],
    cursorHead: [],
    fallbackAnchor: 4,
    fallbackHead: 4,
    scrollAnchor: [],
    fallbackScrollAnchor: 2,
    scrollOffsetPx: 0,
    focusedSurface: 'body',
  };
}

describe('ViewStateController live tab', () => {
  beforeEach(() => {
    stubPageLifecycle();
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('puts the position back when a hidden tab is shown again', () => {
    vi.useFakeTimers();
    const scrollElement = element(0);
    scrollElement.setReach(1000);
    scrollElement.scrollTop = 120;
    const { view, targets, frame } = restoringView(scrollElement);
    const controller = new ViewStateController({
      documentId: 'doc',
      path: () => 'note.md',
      initial: null,
      resolved: resolvedFor(null),
      onChange: vi.fn(),
    });

    controller.attach(view, scrollElement);
    scrollElement.emit('scroll');
    vi.advanceTimersByTime(100);
    while (frame()) {}

    const before = targets.length;
    scrollElement.scrollTop = 0;
    controller.restoreNow();
    while (frame()) {}

    expect(targets.length).toBe(before + 1);
    expect(targets[targets.length - 1]).toMatchObject({ y: 'start' });

    controller.dispose();
    vi.useRealTimers();
  });
});

describe('ViewStateController restore', () => {
  beforeEach(() => {
    stubPageLifecycle();
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('hands the stored line and offset to CodeMirror instead of setting scrollTop itself', () => {
    const scrollElement = element(0);
    scrollElement.setReach(1000);
    const stage = restoringView(scrollElement);
    const controller = new ViewStateController({
      documentId: 'doc',
      path: () => 'note.md',
      initial: { ...storedTop(), scrollOffsetPx: 12 },
      resolved: resolvedFor({ ...storedTop(), scrollOffsetPx: 12 }),
      onChange: vi.fn(),
    });

    controller.attach(stage.view, scrollElement);

    expect(stage.targets).toEqual([
      expect.objectContaining({ y: 'start', yMargin: -12 }),
    ]);
    expect(stage.targets[0].range.head).toBe(2);
    expect(scrollElement.scrollTop).toBe(0);
  });

  it('never stores a position before the restore has been applied', () => {
    const onChange = vi.fn();
    const scrollElement = element(0);
    scrollElement.setReach(1000);
    const stage = restoringView(scrollElement);
    const controller = new ViewStateController({
      documentId: 'doc',
      path: () => 'note.md',
      initial: storedTop(),
      resolved: resolvedFor(storedTop()),
      onChange,
    });

    controller.attach(stage.view, scrollElement);
    scrollElement.emit('scroll');
    controller.update({ selectionSet: false, docChanged: true } as ViewUpdate);

    expect(onChange).not.toHaveBeenCalled();
  });

  it('records the reader scrolling once the restore settled', () => {
    const onChange = vi.fn();
    const scrollElement = element(0);
    scrollElement.setReach(1000);
    const stage = restoringView(scrollElement);
    const controller = new ViewStateController({
      documentId: 'doc',
      path: () => 'note.md',
      initial: storedTop(),
      resolved: resolvedFor(storedTop()),
      onChange,
    });

    controller.attach(stage.view, scrollElement);
    stage.frame();
    scrollElement.scrollTop = 600;
    scrollElement.emit('scroll');
    controller.dispose();

    expect(stage.pending()).toBe(0);
    expect(onChange).toHaveBeenCalledTimes(1);
    expect(onChange.mock.calls[0][0]).toMatchObject({ fallbackScrollAnchor: 2 });
  });

  it('stores the restored position when the tab closes', () => {
    const onChange = vi.fn();
    const scrollElement = element(0);
    scrollElement.setReach(1000);
    const stage = restoringView(scrollElement);
    const controller = new ViewStateController({
      documentId: 'doc',
      path: () => 'note.md',
      initial: storedTop(),
      resolved: resolvedFor(storedTop()),
      onChange,
    });

    controller.attach(stage.view, scrollElement);
    stage.frame();
    controller.dispose(true);

    expect(onChange).toHaveBeenCalledTimes(1);
  });
});
