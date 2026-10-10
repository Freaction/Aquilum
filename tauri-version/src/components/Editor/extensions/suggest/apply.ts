import type { Completion } from '@codemirror/autocomplete';
import type { EditorView } from '@codemirror/view';
import { suggestContext } from './context';

export function applyNoteLink(view: EditorView, completion: Completion) {
  const where = suggestContext(view.state, view.state.selection.main.head);
  if (!where) return;
  const insert = `${where.prefix}${completion.label}${where.suffix}`;
  view.dispatch({
    changes: { from: where.from, to: where.to, insert },
    selection: { anchor: where.from + insert.length },
    userEvent: 'input.complete',
    scrollIntoView: true,
  });
}
