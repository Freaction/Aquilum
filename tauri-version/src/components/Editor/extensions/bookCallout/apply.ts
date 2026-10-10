import { EditorSelection, type EditorState, type TransactionSpec } from '@codemirror/state';
import type { EditorView } from '@codemirror/view';

const BOOK_TITLE_PLACEHOLDER = 'Название книги';
const BOOK_CALLOUT_PREFIX = '> [!book] [[';

export function bookCalloutInsertion(state: EditorState): TransactionSpec {
    const { from, to } = state.selection.main;
    const doc = state.doc;
    const line = doc.lineAt(from);
    const prefix = from > line.from ? '\n' : '';
    const selectedTitle = doc.sliceString(from, to);
    const title = selectedTitle && !selectedTitle.includes('\n')
        ? selectedTitle
        : BOOK_TITLE_PLACEHOLDER;
    const callout = `${BOOK_CALLOUT_PREFIX}${title}]]`;
    const insert = `${prefix}${callout}\n`;
    const titleFrom = from + prefix.length + BOOK_CALLOUT_PREFIX.length;

    return {
        changes: { from, to, insert },
        selection: EditorSelection.range(
            titleFrom,
            titleFrom + title.length,
        ),
        userEvent: 'input',
        scrollIntoView: true,
    };
}

export function insertBookCallout(view: EditorView): boolean {
    if (view.state.readOnly) return false;

    view.dispatch(bookCalloutInsertion(view.state));
    view.focus();
    return true;
}
