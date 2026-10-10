import { EditorView } from '@codemirror/view';

export const tableCellFieldTheme = EditorView.theme({
    '&': {
        width: '100%',
        minHeight: '1.2em',
        boxSizing: 'border-box',
        padding: '0',
        backgroundColor: 'transparent',
        font: 'inherit',
        color: 'inherit',
        lineHeight: 'inherit',
        userSelect: 'text',
        '--q-cm-content-padding': '0',
    },
    '&.cm-focused': {
        outline: 'none',
    },
    '.cm-scroller': {
        overflow: 'hidden',
        boxSizing: 'border-box',
        padding: '0',
        font: 'inherit',
        lineHeight: 'inherit',
    },
    '.cm-content': {
        width: '100%',
        boxSizing: 'border-box',
        minHeight: '1.2em',
        caretColor: 'transparent',
        whiteSpace: 'pre-wrap',
        overflowWrap: 'anywhere',
        wordBreak: 'break-word',
        font: 'inherit',
        color: 'inherit',
        lineHeight: 'inherit',
        userSelect: 'text',
    },
    '.cm-line': {
        padding: '0',
        lineHeight: 'inherit',
    },
});
