import { EditorView } from '@codemirror/view';

const clip = {
  minWidth: '0',
  overflow: 'hidden',
  textOverflow: 'ellipsis',
  whiteSpace: 'nowrap',
} as const;

export const bookCalloutTheme = EditorView.baseTheme({
  '.q-md-book-callout-group': {
    width: 'var(--q-editor-block-width, 100%)',
    display: 'flex',
    flexDirection: 'column',
    boxSizing: 'border-box',
    minWidth: '0',
    overflow: 'hidden',
    borderRadius: 'var(--q-block-radius)',
    border: 'var(--q-border-1) solid var(--q-block-border)',
    backgroundColor: 'var(--q-block-bg)',
  },
  '.q-md-book-callout-group > .q-md-book-callout': {
    padding: 'var(--q-gap-xl)',
    border: '0',
    borderBottom: 'var(--q-border-1) solid var(--q-block-divider)',
    borderRadius: '0',
    backgroundColor: 'transparent',
  },
  '.q-md-book-callout-group > .q-md-book-callout:last-child': {
    borderBottom: '0',
  },
  '.q-md-book-callout': {
    display: 'flex',
    alignItems: 'center',
    gap: 'var(--q-gap-xl)',
    boxSizing: 'border-box',
    minWidth: '0',
    overflow: 'hidden',
    padding: 'var(--q-gap-md)',
    borderRadius: 'var(--q-block-radius)',
    border: 'var(--q-border-1) solid var(--q-block-border)',
    backgroundColor: 'var(--q-block-bg)',
  },
  '.q-md-book-callout--pending': {
    minHeight: '72px',
  },
  '.q-md-book-callout-cover': {
    flex: '0 0 auto',
    width: '56px',
    height: '72px',
    borderRadius: 'var(--q-radius-lg)',
    overflow: 'hidden',
    backgroundColor: 'var(--q-book-callout-cover-bg)',
  },
  '.q-md-book-callout-cover img': {
    display: 'block',
    width: '100%',
    height: '100%',
    objectFit: 'cover',
  },
  '.q-md-book-callout-text': {
    display: 'flex',
    flexDirection: 'column',
    justifyContent: 'center',
    flex: '1 1 auto',
    minWidth: '0',
    fontFamily: 'var(--q-editor-font-family)',
    fontSize: 'var(--q-editor-font-size)',
    lineHeight: 'var(--q-editor-line-height)',
    color: 'var(--q-editor-text)',
  },
  '.q-md-book-callout-title': {
    ...clip,
    font: 'inherit',
    color: 'inherit',
  },
  '.q-md-book-callout-title--link': {
    color: 'var(--q-text-accent)',
    cursor: 'pointer',
    textDecoration: 'none',
  },
  '.q-md-book-callout-title--link:hover': {
    textDecoration: 'underline',
  },
  '.q-md-book-callout-author': {
    ...clip,
    font: 'inherit',
    color: 'var(--q-text-muted)',
  },
  '.q-md-book-callout-right': {
    display: 'flex',
    flexDirection: 'column',
    alignItems: 'flex-end',
    justifyContent: 'space-between',
    flex: '0 0 auto',
    alignSelf: 'stretch',
    fontFamily: 'var(--q-font-family-ui)',
    fontSize: 'var(--q-font-size-ui-base)',
    fontWeight: 'var(--q-font-weight-ui-base)',
    lineHeight: 'normal',
  },
  '.q-md-book-callout-read': {
    flex: '0 0 auto',
  },
  '.q-md-book-callout-read:disabled': {
    opacity: 'var(--q-opacity-50)',
    cursor: 'default',
  },
  '.q-md-book-callout-progress': {
    font: 'inherit',
    color: 'var(--q-text-muted)',
    whiteSpace: 'nowrap',
  },
  '.q-md-book-callout-progress--done': {
    color: 'var(--q-text-success)',
  },
});
