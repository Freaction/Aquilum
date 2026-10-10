import { EditorSelection, EditorState, Prec, Transaction, type Text } from '@codemirror/state';
import { EditorView, keymap, ViewPlugin } from '@codemirror/view';
import {
    findTablesInDoc,
    tableOwnsDocEnd,
    type TableRange,
} from './constructs';

export function blankLineInsertPos(doc: Text, table: TableRange): number | null {
    if (table.contentTo <= table.from) return null;
    if (tableOwnsDocEnd(doc, table)) return doc.length;

    const lastLine = doc.lineAt(table.contentTo - 1);
    const next = doc.line(lastLine.number + 1);
    if (next.text.length > 0) return next.from;
    return null;
}

export function collectBlankLineChanges(doc: Text): { from: number; insert: string }[] {
    const tables = findTablesInDoc(doc);
    const changes: { from: number; insert: string }[] = [];
    for (let i = tables.length - 1; i >= 0; i--) {
        const from = blankLineInsertPos(doc, tables[i]);
        if (from != null) changes.push({ from, insert: '\n' });
    }
    return changes;
}

const tableBlankLineFilter = EditorState.transactionFilter.of((tr) => {
    if (!tr.docChanged) return tr;
    const changes = collectBlankLineChanges(tr.newDoc);
    if (changes.length === 0) return tr;
    return [tr, { changes, sequential: true }];
});

const tableBlankLineInit = ViewPlugin.fromClass(class {
    constructor(view: EditorView) {
        const changes = collectBlankLineChanges(view.state.doc);
        if (changes.length === 0) return;
        view.dispatch({
            changes,
            annotations: [Transaction.addToHistory.of(false)],
        });
    }
});

export const tableCaretGuard = EditorState.transactionFilter.of((tr) => {
    if (!tr.selection) return tr;
    const sel = tr.state.selection.main;
    if (!sel.empty) return tr;
    for (const table of findTablesInDoc(tr.state.doc)) {
        if (sel.head >= table.from && sel.head < table.to) {
            return [
                tr,
                {
                    selection: { anchor: table.to, head: table.to },
                    sequential: true,
                    annotations: [Transaction.addToHistory.of(false)],
                },
            ];
        }
    }
    return tr;
});

export function docStartsWithTable(state: EditorState): boolean {
    return findTablesInDoc(state.doc)[0]?.from === 0;
}

export function tableToSelectOnBackspace(state: EditorState): TableRange | null {
    if (state.readOnly) return null;
    const sel = state.selection.main;
    if (!sel.empty) return null;
    const pos = sel.head;
    if (pos === 0) return null;

    for (const table of findTablesInDoc(state.doc)) {
        if (
            pos === table.to
            || pos === table.contentTo
            || pos === table.contentTo + 1
        ) {
            return table;
        }
    }
    return null;
}

function backspaceAtTableBoundary(view: EditorView): boolean {
    if (view.dom.classList.contains('q-md-table-cell-focused')) return false;
    const table = tableToSelectOnBackspace(view.state);
    if (!table) return false;
    view.dispatch({
        selection: EditorSelection.range(table.from, table.to),
        userEvent: 'select',
    });
    return true;
}

export const tableBoundaryExtension = [
    tableBlankLineFilter,
    tableBlankLineInit,
    tableCaretGuard,
    Prec.high(keymap.of([{ key: 'Backspace', run: backspaceAtTableBoundary }])),
];
