import { EditorSelection, type EditorState } from '@codemirror/state';
import type { EditorView } from '@codemirror/view';
import { noteFrontmatterTemplate } from '../../../modules/docs/frontmatter';
import { stateFrontmatterRange } from './frontmatterUi';

export function frontmatterCaretOffset(template: string): number {
  const index = template.indexOf('[]');
  return index === -1 ? template.length : index + 1;
}

export function frontmatterInsertText(state: EditorState, template: string): string {
  return state.doc.length > 0 ? `${template}\n` : template;
}

export function canInsertFrontmatter(state: EditorState): boolean {
  if (state.readOnly) return false;
  return stateFrontmatterRange(state) === null;
}

export function insertFrontmatter(view: EditorView): boolean {
  if (!canInsertFrontmatter(view.state)) return false;
  const template = noteFrontmatterTemplate();
  view.dispatch({
    changes: { from: 0, insert: frontmatterInsertText(view.state, template) },
    selection: EditorSelection.cursor(frontmatterCaretOffset(template)),
    scrollIntoView: true,
  });
  view.focus();
  return true;
}
