import { EditorView } from '@codemirror/view';

export const codeBlockTheme = EditorView.theme({
  '.cm-line.q-md-code-line': {
    position: 'relative',
    boxSizing: 'border-box',
    fontFamily: 'var(--q-editor-code-font)',
    paddingLeft: 'var(--q-block-padding)',
    paddingRight: 'var(--q-block-padding)',
  },
  '.cm-line.q-md-code-line::before': {
    content: '""',
    position: 'absolute',
    inset: '0',
    zIndex: '-2',
    borderInline: 'var(--q-border-1) solid var(--q-block-border)',
    backgroundColor: 'var(--q-block-bg)',
    pointerEvents: 'none',
  },
  '.cm-line.q-md-code-line--first': {
    paddingTop: 'var(--q-block-padding)',
  },
  '.cm-line.q-md-code-line--first::before': {
    borderTop: 'var(--q-border-1) solid var(--q-block-border)',
    borderTopLeftRadius: 'var(--q-block-radius)',
    borderTopRightRadius: 'var(--q-block-radius)',
  },
  '.cm-line.q-md-code-line--last': {
    paddingBottom: 'var(--q-block-padding)',
  },
  '.cm-line.q-md-code-line--last::before': {
    borderBottom: 'var(--q-border-1) solid var(--q-block-border)',
    borderBottomLeftRadius: 'var(--q-block-radius)',
    borderBottomRightRadius: 'var(--q-block-radius)',
  },
  '.cm-line.q-md-code-line .q-md-code': {
    padding: '0',
    borderRadius: '0',
    backgroundColor: 'transparent',
  },
  '.cm-line.q-md-code-line .q-code-keyword': { color: 'var(--q-editor-code-keyword)' },
  '.cm-line.q-md-code-line .q-code-string': { color: 'var(--q-editor-code-string)' },
  '.cm-line.q-md-code-line .q-code-number': { color: 'var(--q-editor-code-number)' },
  '.cm-line.q-md-code-line .q-code-comment': { color: 'var(--q-editor-code-comment)', fontStyle: 'italic' },
  '.cm-line.q-md-code-line .q-code-function': { color: 'var(--q-editor-code-function)' },
  '.cm-line.q-md-code-line .q-code-type': { color: 'var(--q-editor-code-type)' },
  '.cm-line.q-md-code-line .q-code-property': { color: 'var(--q-editor-code-property)' },
});
