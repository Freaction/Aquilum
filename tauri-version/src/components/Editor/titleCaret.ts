import { EditorSelection } from '@codemirror/state';
import type { EditorView } from '@codemirror/view';
import { docStartsWithTable } from './extensions/tables/boundary';

export function isOnLastVisualLine(view: EditorView): boolean {
  const range = view.state.selection.main;
  return range.empty && view.moveVertically(range, true).head === range.head;
}

export function bodyPositionUnderCaret(title: EditorView, body: EditorView): number {
  const caret = title.coordsAtPos(title.state.selection.main.head);
  const firstLine = body.coordsAtPos(0);
  if (!caret || !firstLine) return 0;
  return body.posAtCoords({ x: caret.left, y: (firstLine.top + firstLine.bottom) / 2 }) ?? 0;
}

export function focusBodyAt(body: EditorView, position: number): void {
  body.dispatch({ selection: EditorSelection.cursor(position), scrollIntoView: true });
  body.focus();
}

export function enterBodyFromTitle(body: EditorView): void {
  if (!docStartsWithTable(body.state)) {
    focusBodyAt(body, 0);
    return;
  }
  body.dispatch({
    changes: { from: 0, insert: '\n' },
    selection: EditorSelection.cursor(0),
    scrollIntoView: true,
  });
  body.focus();
}

export function hasNoModifiers(event: KeyboardEvent): boolean {
  return !event.shiftKey && !event.altKey && !event.ctrlKey && !event.metaKey;
}
