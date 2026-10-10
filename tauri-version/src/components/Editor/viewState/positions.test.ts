import { describe, expect, it } from 'vitest';
import { fallbackView, initialSelection } from './positions';

const stored = {
  documentId: 'doc', paneId: 'main', cursorAnchor: [], cursorHead: [],
  fallbackAnchor: 3, fallbackHead: 5, scrollAnchor: [],
  fallbackScrollAnchor: 1, scrollOffsetPx: 0, focusedSurface: 'body',
};

describe('view state positions', () => {
  it('falls back to the stored offsets when the document cannot resolve a position', () => {
    expect(fallbackView(stored)).toEqual({ anchor: 3, head: 5, scroll: 1 });
  });

  it('builds the initial CodeMirror selection before mounting', () => {
    expect(initialSelection(fallbackView(stored), 7)).toEqual({ anchor: 3, head: 5 });
    expect(initialSelection(fallbackView(stored), 7, 6)).toEqual({ anchor: 6, head: 6 });
  });

  it('clamps positions into a document that became shorter', () => {
    expect(initialSelection({ anchor: 40, head: 50, scroll: 0 }, 7)).toEqual({ anchor: 7, head: 7 });
  });

  it('has no selection to restore without a saved view', () => {
    expect(initialSelection(null, 7)).toBeUndefined();
  });
});
