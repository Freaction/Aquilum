import { syntaxTree } from '@codemirror/language';
import { EditorSelection, type EditorState } from '@codemirror/state';
import { EditorView, keymap } from '@codemirror/view';
import { editablePreviewBlockEntryPos, selectionInEditablePreviewBlock } from '../livePreviewEditableSpans';
import { LINK_OWNERS } from '../livePreviewReveal';
import { findAncestor } from '../syntaxAncestor';

export function inlineLinkOwnerAt(
  state: EditorState,
  pos: number,
): { from: number; to: number } | null {
  const clamped = Math.max(0, Math.min(pos, state.doc.length));
  const tree = syntaxTree(state);
  const owner = findAncestor(tree.resolveInner(clamped, 0), (node) => LINK_OWNERS.includes(node.name))
    ?? (clamped > 0
      ? findAncestor(
        tree.resolveInner(clamped - 1, 1),
        (node) => LINK_OWNERS.includes(node.name) && clamped <= node.to,
      )
      : null);
  return owner ? { from: owner.from, to: owner.to } : null;
}

export function docLineStepPos(
  state: EditorState,
  head: number,
  forward: boolean,
): number | null {
  const line = state.doc.lineAt(head);
  const col = head - line.from;
  if (forward) {
    if (line.number >= state.doc.lines) return null;
    const next = state.doc.line(line.number + 1);
    return Math.min(next.from + col, next.to);
  }
  if (line.number <= 1) return null;
  const prev = state.doc.line(line.number - 1);
  return Math.min(prev.from + col, prev.to);
}

function movePreviewBlockAwareVertical(view: EditorView, forward: boolean): boolean {
  const { state } = view;
  const main = state.selection.main;
  if (!main.empty) return false;

  const caretLine = state.doc.lineAt(main.head).number;
  const pos = editablePreviewBlockEntryPos(state, caretLine, forward);
  if (pos == null) return false;

  view.dispatch({
    selection: EditorSelection.cursor(pos),
    scrollIntoView: true,
  });
  return true;
}

function moveLinkAwareVertical(view: EditorView, forward: boolean): boolean {
  const { state } = view;
  const main = state.selection.main;
  if (!main.empty) return false;
  if (selectionInEditablePreviewBlock(state)) return false;
  if (!inlineLinkOwnerAt(state, main.head)) return false;

  const curLine = state.doc.lineAt(main.head);
  const tentative = view.moveVertically(main, forward);
  if (state.doc.lineAt(tentative.head).number !== curLine.number) {
    return false;
  }

  const pos = docLineStepPos(state, main.head, forward);
  if (pos == null) return true;

  view.dispatch({
    selection: EditorSelection.cursor(pos, undefined, undefined, tentative.goalColumn ?? undefined),
    scrollIntoView: true,
  });
  return true;
}

function moveVertical(view: EditorView, forward: boolean): boolean {
  return movePreviewBlockAwareVertical(view, forward)
    || moveLinkAwareVertical(view, forward);
}

export const markdownLinkMotion = keymap.of([
  { key: 'ArrowDown', run: (view) => moveVertical(view, true) },
  { key: 'ArrowUp', run: (view) => moveVertical(view, false) },
]);
