import { EditorView } from '@codemirror/view';
import { PENDING_HEIGHT } from './constants';

export const dataviewTheme = EditorView.baseTheme({
  '.q-md-dataview': {
    display: 'block',
    boxSizing: 'border-box',
    width: '100%',
  },
  '.q-md-dataview-card': {
    boxSizing: 'border-box',
    minWidth: '0',
    overflowX: 'auto',
    padding: 'var(--q-block-padding)',
    borderRadius: 'var(--q-block-radius)',
    border: 'var(--q-border-1) solid var(--q-block-border)',
    backgroundColor: 'var(--q-block-bg)',
    fontFamily: 'var(--q-editor-font-family)',
    fontSize: 'var(--q-editor-font-size)',
    lineHeight: 'var(--q-editor-line-height)',
  },
  '.q-md-dataview-card--pending': {
    minHeight: PENDING_HEIGHT,
  },
  '.q-md-dataview-title': {
    padding: 'var(--q-gap-sm) 0',
    borderBottom: 'var(--q-border-1) solid var(--q-block-divider)',
    color: 'var(--q-text-secondary)',
    fontWeight: 'var(--q-font-weight-bold)',
  },
  '.q-md-dataview-title--cells': {
    paddingInline: 'var(--q-gap-md)',
  },
  '.q-md-dataview-table': {
    width: '100%',
    borderCollapse: 'collapse',
  },
  '.q-md-dataview-table th': {
    textAlign: 'left',
    padding: 'var(--q-gap-sm) var(--q-gap-md)',
    borderBottom: 'var(--q-border-1) solid var(--q-block-divider)',
    color: 'var(--q-text-secondary)',
    fontWeight: 'var(--q-font-weight-bold)',
    whiteSpace: 'nowrap',
  },
  '.q-md-dataview-table td': {
    padding: 'var(--q-gap-sm) var(--q-gap-md)',
    borderBottom: 'var(--q-border-1) solid var(--q-block-divider)',
    verticalAlign: 'top',
  },
  '.q-md-dataview-table td.q-md-dataview-cell-bar': {
    width: '40%',
    verticalAlign: 'middle',
  },
  '.q-md-dataview-table tbody tr:last-child td': {
    borderBottom: '0',
  },
  '.q-md-dataview-list': {
    margin: '0',
    paddingLeft: 'var(--q-gap-xl)',
  },
  '.q-md-dataview-list li': {
    padding: 'var(--q-gap-xs) 0',
  },
  '.q-md-dataview-link': {
    color: 'var(--q-text-link)',
    cursor: 'pointer',
  },
  '.q-md-dataview-link:hover': {
    textDecoration: 'underline',
  },
  '.q-md-dataview-tasks': {
    margin: '0',
    padding: '0',
    listStyle: 'none',
  },
  '.q-md-dataview-tasks li': {
    display: 'flex',
    alignItems: 'center',
    padding: 'var(--q-gap-xs) 0',
  },
  '.q-md-dataview-tasks .q-task-check': {
    marginInlineEnd: 'var(--q-space-12)',
  },
  '.q-md-dataview-bar': {
    minWidth: '120px',
    height: 'var(--q-unit-8)',
    borderRadius: 'var(--q-radius-full)',
    backgroundColor: 'var(--q-bg-accent-subtle)',
    overflow: 'hidden',
  },
  '.q-md-dataview-bar-fill': {
    height: '100%',
    borderRadius: 'inherit',
    backgroundColor: 'var(--q-bg-accent)',
  },
  '.q-md-dataview-message': {
    marginTop: 'var(--q-gap-sm)',
    color: 'var(--q-text-secondary)',
  },
  '.q-md-dataview-card--failed .q-md-dataview-message': {
    marginTop: '0',
    color: 'var(--q-text-danger)',
  },
});
