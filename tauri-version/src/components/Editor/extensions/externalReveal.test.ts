import { describe, expect, it } from 'vitest';
import { EditorState } from '@codemirror/state';
import { EditorView, type DecorationSet } from '@codemirror/view';
import {
  externalRevealExtension,
  revealExternalInsert,
  tickExternalReveal,
} from './externalReveal';

function editor(doc: string) {
  let state = EditorState.create({ doc, extensions: [externalRevealExtension] });
  const view = {
    get state() {
      return state;
    },
    dispatch(spec: Parameters<EditorState['update']>[0]) {
      state = state.update(spec).state;
    },
  } as unknown as EditorView;

  const hidden = (): Array<{ from: number; to: number }> => {
    const ranges: Array<{ from: number; to: number }> = [];
    const [set] = state.facet(EditorView.decorations) as DecorationSet[];
    set.between(0, state.doc.length, (from, to) => {
      ranges.push({ from, to });
    });
    return ranges;
  };

  return { view, hidden, current: () => state };
}

const AGENT_TEXT = 'текст от агента';

describe('externalReveal', () => {
  it('hides the whole inserted range and uncovers it step by step', () => {
    const { view, hidden } = editor(`начало ${AGENT_TEXT} конец`);
    revealExternalInsert(view, { from: 7, to: 7, insert: AGENT_TEXT });

    expect(hidden()).toEqual([{ from: 7, to: 7 + AGENT_TEXT.length }]);

    tickExternalReveal(view);
    const [afterTick] = hidden();
    expect(afterTick.from).toBeGreaterThan(7);
    expect(afterTick.to).toBe(7 + AGENT_TEXT.length);
  });

  it('stops hiding anything once the text is fully shown', () => {
    const { view, hidden } = editor(`начало ${AGENT_TEXT}`);
    revealExternalInsert(view, { from: 7, to: 7, insert: AGENT_TEXT });

    for (let step = 0; step < AGENT_TEXT.length + 1; step += 1) {
      tickExternalReveal(view);
    }
    expect(hidden()).toEqual([]);
  });

  it('follows the text when the user types before it', () => {
    const { view, hidden, current } = editor(`начало ${AGENT_TEXT}`);
    revealExternalInsert(view, { from: 7, to: 7, insert: AGENT_TEXT });
    view.dispatch(current().update({ changes: { from: 0, insert: 'ввод ' } }));

    expect(hidden()).toEqual([{ from: 12, to: 12 + AGENT_TEXT.length }]);
  });

  it('shows replacements and oversized inserts instantly', () => {
    const replacement = editor('старый текст');
    revealExternalInsert(replacement.view, { from: 0, to: 6, insert: 'новый' });
    expect(replacement.hidden()).toEqual([]);

    const huge = 'а'.repeat(2_001);
    const oversized = editor(huge);
    revealExternalInsert(oversized.view, { from: 0, to: 0, insert: huge });
    expect(oversized.hidden()).toEqual([]);
  });

  it('ignores a range that no longer fits the document', () => {
    const { view, hidden } = editor('коротко');
    revealExternalInsert(view, { from: 0, to: 0, insert: 'длиннее чем документ' });
    expect(hidden()).toEqual([]);
  });
});
