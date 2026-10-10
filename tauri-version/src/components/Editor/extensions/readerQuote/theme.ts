import { EditorView } from '@codemirror/view';

export const readerQuoteTheme = EditorView.baseTheme({
  '.q-md-reader-quote': {
    display: 'block',
    width: '100%',
    maxWidth: '100%',
    fontFamily: 'var(--q-editor-font-family)',
    fontSize: 'var(--q-editor-font-size)',
    lineHeight: 'var(--q-editor-line-height)',
    color: 'var(--q-editor-text)',
  },
  '.q-md-reader-quote-text': {
    font: 'inherit',
    color: 'inherit',
    whiteSpace: 'pre-wrap',
  },
  '.q-md-reader-quote-ref': {
    display: 'inline',
    padding: '0',
    border: 'none',
    background: 'none',
    font: 'inherit',
    lineHeight: 'inherit',
    color: 'var(--q-text-accent)',
    cursor: 'pointer',
    textDecoration: 'none',
    verticalAlign: 'baseline',
  },
  '.q-md-reader-quote-ref:hover': {
    textDecoration: 'underline',
  },
});
