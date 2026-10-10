import { Compartment, EditorState } from '@codemirror/state';
import { EditorView, ViewPlugin } from '@codemirror/view';
import { getReadingMode, subscribeReadingMode } from '../../../../plugins/editor/readingMode';
import { revealAtCaretFacet } from '../livePreviewConfig';

export function readingModeExtension() {
  const mode = new Compartment();
  const flags = () => [
    EditorState.readOnly.compute([], getReadingMode),
    EditorView.editable.compute([], () => !getReadingMode()),
    revealAtCaretFacet.compute([], () => !getReadingMode()),
  ];
  return [
    mode.of(flags()),
    // Виджеты могут отправлять изменения напрямую; collab применяет входящие с filter: false.
    EditorState.changeFilter.of(tr => !tr.startState.readOnly),
    ViewPlugin.define(view => {
      const unsubscribe = subscribeReadingMode(() => view.dispatch({
        effects: mode.reconfigure(flags()),
        // Пересчитываем reveal и виджеты без изменения позиции каретки.
        selection: view.state.selection,
      }));
      return { destroy: unsubscribe };
    }),
  ];
}
