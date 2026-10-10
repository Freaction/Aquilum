import { ensureSyntaxTree, syntaxTree } from '@codemirror/language';
import type { EditorView } from '@codemirror/view';
import type { Tree } from '@lezer/common';

const PARSE_BUDGET_MS = 6;

export function ensureEditorTree(view: EditorView, allowParse = true): Tree {
  const existing = syntaxTree(view.state);
  if (!allowParse) return existing;
  let upto = view.state.selection.main.head;
  for (const range of view.visibleRanges) upto = Math.max(upto, range.to);
  upto = Math.min(view.state.doc.length, upto);
  if (existing.length >= upto) return existing;
  return ensureSyntaxTree(view.state, upto, PARSE_BUDGET_MS) ?? existing;
}

export function visibleTreeRanges(
  view: EditorView,
  treeLength: number,
): readonly { from: number; to: number }[] {
  if (view.visibleRanges.length > 0) return view.visibleRanges;
  return [{ from: 0, to: Math.min(treeLength, view.state.doc.length) }];
}
