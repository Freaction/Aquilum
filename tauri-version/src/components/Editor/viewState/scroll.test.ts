import { describe, expect, it } from 'vitest';
import type { EditorView } from '@codemirror/view';
import { captureScroll, resolveEditorScrollElement } from './scroll';

function element(top: number): HTMLElement {
  return {
    getBoundingClientRect: () => ({ top }),
  } as unknown as HTMLElement;
}

describe('continuous document scrolling', () => {
  it('measures the anchor against the same character rect CodeMirror scrolls to', () => {
    const scroll = element(0);
    const view = {
      scrollDOM: element(-380),
      lineBlockAtHeight: () => ({ from: 100, top: 360 }),
      coordsAtPos: () => ({ top: -18 }),
    } as unknown as EditorView;
    expect(captureScroll(view, scroll)).toEqual({ anchor: 100, offset: 18 });
  });

  it('keeps the offset negative while the cover still pushes the first line down', () => {
    const scroll = element(0);
    const view = {
      scrollDOM: element(70),
      lineBlockAtHeight: () => ({ from: 0, top: 0 }),
      coordsAtPos: () => ({ top: 72 }),
    } as unknown as EditorView;
    expect(captureScroll(view, scroll)).toEqual({ anchor: 0, offset: -72 });
  });

  it('falls back to the line block top when the anchor is not rendered', () => {
    const scroll = element(0);
    const view = {
      scrollDOM: element(-380),
      lineBlockAtHeight: () => ({ from: 100, top: 360 }),
      coordsAtPos: () => null,
    } as unknown as EditorView;
    expect(captureScroll(view, scroll)).toEqual({ anchor: 100, offset: 20 });
  });

  it('uses active book page as scrollport when present', () => {
    const book = { className: 'q-book-page' } as unknown as HTMLElement;
    const container = {
      querySelector: (selector: string) => (
        selector === '.q-book-page:not(.q-book-page--inactive)' ? book : null
      ),
    } as unknown as HTMLElement;
    expect(resolveEditorScrollElement(container)).toBe(book);
  });

  it('falls back to container when book page is inactive', () => {
    const container = {
      querySelector: () => null,
    } as unknown as HTMLElement;
    expect(resolveEditorScrollElement(container)).toBe(container);
  });
});
