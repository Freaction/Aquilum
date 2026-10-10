import { history, defaultKeymap, historyKeymap } from '@codemirror/commands';
import { EditorState, type Extension } from '@codemirror/state';
import { drawSelection, EditorView, keymap } from '@codemirror/view';

const aquilumFieldCore: Extension = [
  history(),
  keymap.of([...defaultKeymap, ...historyKeymap]),
];

export const aquilumFieldSetup: Extension = [
  aquilumFieldCore,
  drawSelection(),
];

export const aquilumCodeMirrorTheme = EditorView.theme({
  '&': {
    color: 'var(--q-cm-text-color, var(--q-text-primary))',
    backgroundColor: 'transparent',
    fontFamily: 'var(--q-cm-font-family, var(--q-font-family-ui))',
    fontSize: 'var(--q-cm-font-size, var(--q-font-size-ui-base))',
    fontWeight: 'var(--q-cm-font-weight, var(--q-font-weight-ui-base))',
    lineHeight: 'var(--q-cm-line-height, var(--q-font-line-height-normal))',
    letterSpacing: '0',
  },
  '&.cm-focused': {
    outline: 'none',
  },
  '.cm-scroller': {
    fontFamily: 'inherit',
    fontWeight: 'inherit',
    fontSize: 'inherit',
    lineHeight: 'inherit',
  },
  '.cm-content': {
    padding: 'var(--q-cm-content-padding, 0 var(--q-caret-width))',
    caretColor: 'transparent',
  },
  '.cm-line': {
    padding: '0',
  },
  '.cm-cursor, .cm-dropCursor': {
    borderLeft: 'var(--q-caret-width) solid var(--q-caret-color)',
    borderRadius: 'var(--q-caret-radius)',
    transform: 'scaleY(var(--q-caret-scale-y))',
  },
  '.cm-activeLine': {
    backgroundColor: 'transparent',
  },
  '.cm-placeholder': {
    color: 'var(--q-cm-placeholder-color, var(--q-text-tertiary))',
  },
  '.cm-selectionBackground, &.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground': {
    backgroundColor: 'var(--q-selection-bg)',
  },
  '.cm-selectionMatch': {
    backgroundColor: 'var(--q-selection-match-bg)',
  },
  '.cm-selectionMatch.cm-selectionMatch-main': {
    backgroundColor: 'transparent',
  },
  '& .cm-content::selection, & .cm-content *::selection, & .cm-line::selection, & .cm-line *::selection, &.cm-focused .cm-content:focus::selection, &.cm-focused .cm-content:focus *::selection, &.cm-focused .cm-line::selection, &.cm-focused .cm-line *::selection': {
    backgroundColor: 'transparent !important',
  },
});

export function syncDocument(view: EditorView, next: string): void {
  const current = view.state.doc.toString();
  if (current === next) return;
  if (view.composing) {
    view.contentDOM.addEventListener(
      'compositionend',
      () => syncDocument(view, next),
      { once: true },
    );
    return;
  }

  const limit = Math.min(current.length, next.length);
  let head = 0;
  while (head < limit && current.charCodeAt(head) === next.charCodeAt(head)) head += 1;
  let tail = 0;
  while (
    tail < limit - head
    && current.charCodeAt(current.length - 1 - tail) === next.charCodeAt(next.length - 1 - tail)
  ) tail += 1;

  view.dispatch({
    changes: {
      from: head,
      to: current.length - tail,
      insert: next.slice(head, next.length - tail),
    },
  });
}

export function normalizeSingleLineText(text: string): string {
  return text
    .replace(/[ \t]*(?:(?:\r\n?|[\n\u2028\u2029])[ \t]*)+/g, ' ')
    .replace(/\t+/g, ' ');
}

export const aquilumHorizontalFieldTheme = EditorView.theme({
  '.cm-scroller': {
    overflowX: 'auto',
    overflowY: 'hidden',
    scrollbarWidth: 'none',
  },
  '.cm-scroller::-webkit-scrollbar': {
    display: 'none',
  },
});

export const aquilumSingleLineExtensions: Extension = [
  EditorView.clipboardInputFilter.of(normalizeSingleLineText),
  EditorState.transactionFilter.of((transaction) => (
    transaction.docChanged && transaction.newDoc.lines > 1 ? [] : transaction
  )),
];
