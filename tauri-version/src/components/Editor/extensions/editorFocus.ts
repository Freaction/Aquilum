import { type EditorState, type Extension, StateEffect, StateField } from '@codemirror/state';
import { EditorView, ViewPlugin, type ViewUpdate } from '@codemirror/view';

export const setEditorFocus = StateEffect.define<boolean>();

const editorFocusField = StateField.define<boolean>({
  create: () => false,
  update: (value, tr) => {
    for (const effect of tr.effects) {
      if (effect.is(setEditorFocus)) return effect.value;
    }
    return value;
  },
});

export function editorHasFocus(state: EditorState): boolean {
  return state.field(editorFocusField, false) ?? false;
}

function syncFocus(view: EditorView): void {
  queueMicrotask(() => {
    if (editorHasFocus(view.state) === view.hasFocus) return;
    view.dispatch({ effects: setEditorFocus.of(view.hasFocus) });
  });
}

const focusWatcher = ViewPlugin.fromClass(class {
  private readonly view: EditorView;

  constructor(view: EditorView) {
    this.view = view;
    syncFocus(view);
  }

  update(update: ViewUpdate) {
    if (update.focusChanged) syncFocus(this.view);
  }
});

export function collapseSelection(view: EditorView): void {
  const { main } = view.state.selection;
  if (main.empty) return;
  view.dispatch({ selection: { anchor: main.head } });
}

export function editorFocusExtension(): Extension {
  return [editorFocusField, focusWatcher];
}
