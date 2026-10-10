import type { EditorView } from '@codemirror/view';
import { samePath } from '../../modules/paths';

interface OpenEditor {
  path: () => string;
  view: EditorView;
}

const editors = new Set<OpenEditor>();

export function registerEditor(path: () => string, view: EditorView): () => void {
  const editor = { path, view };
  editors.add(editor);
  return () => {
    editors.delete(editor);
  };
}

export function editorFor(path: string): EditorView | null {
  for (const editor of editors) {
    if (samePath(editor.path(), path)) return editor.view;
  }
  return null;
}
