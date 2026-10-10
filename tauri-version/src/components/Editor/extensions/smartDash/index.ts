import type { Extension } from '@codemirror/state';
import { EditorView } from '@codemirror/view';

export function smartDashExtension(enabled: boolean): Extension {
  if (!enabled) return [];
  return EditorView.inputHandler.of((view, from, to, text) => {
    if (text !== ' ' || from !== to) return false;
    const before = view.state.sliceDoc(Math.max(0, from - 3), from);
    if (!before.endsWith('--') || before.endsWith('---')) return false;
    view.dispatch({
      changes: { from: from - 2, to: from, insert: '— ' },
      selection: { anchor: from },
      userEvent: 'input.type',
    });
    return true;
  });
}
