import { EditorView } from '@codemirror/view';

export const outlineTheme = EditorView.theme({
    '.q-md-outline-bullet': {
        display: 'inline-flex',
        alignItems: 'center',
        justifyContent: 'flex-start',
        width: 'var(--q-editor-list-marker-width)',
        height: '1em',
        userSelect: 'none',
        verticalAlign: 'middle',
    },
    '.q-md-outline-bullet::after': {
        content: '""',
        width: 'var(--q-space-5)',
        height: 'var(--q-space-5)',
        marginInlineStart: 'calc((1ch - var(--q-space-5)) / 2)',
        borderRadius: '50%',
        backgroundColor: 'var(--q-editor-list-marker-color)',
    },
    '.q-md-outline-mark, .q-md-outline-mark .q-md-syntax-char': {
        color: 'var(--q-editor-list-marker-color)',
    },
    '.q-md-outline-mark': {
        display: 'inline-block',
        minInlineSize: 'var(--q-editor-list-marker-width)',
        textIndent: '0',
    },
    '.q-md-outline-mark-plain, .q-md-outline-mark-plain .q-md-syntax-char': {
        color: 'var(--q-editor-text)',
    },
    '.q-md-list-item': {
        paddingInlineStart: 'calc(var(--q-editor-list-padding-inline-start) + var(--q-editor-list-level-indent) + var(--q-editor-list-marker-width))',
        textIndent: 'calc(0px - var(--q-editor-list-level-indent) - var(--q-editor-list-marker-width))',
    },
    '.q-md-list-continuation': {
        textIndent: '0',
    },
    '.q-md-list-indent': {
        display: 'inline-block',
        inlineSize: 'var(--q-editor-list-level-indent)',
        minInlineSize: 'var(--q-editor-list-level-indent)',
        blockSize: '1em',
        lineHeight: '1',
        overflow: 'hidden',
        whiteSpace: 'pre',
        tabSize: 'var(--q-editor-list-indent-width)',
        textIndent: '0',
        verticalAlign: 'text-bottom',
    },
});
