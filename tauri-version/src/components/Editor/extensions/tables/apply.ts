import { EditorSelection } from '@codemirror/state';
import type { EditorView } from '@codemirror/view';
import { serializeTable } from './constructs';
import { createEmptyTableModel, type CellRef, type TableModel } from './model';

interface PendingTableFocus extends CellRef {
    tableFrom: number;
}

const pendingFocus = new WeakMap<EditorView, PendingTableFocus>();

export function takeTableCellFocus(view: EditorView): PendingTableFocus | null {
    const focus = pendingFocus.get(view) ?? null;
    pendingFocus.delete(view);
    return focus;
}

function queueTableCellFocus(
    view: EditorView,
    tableFrom: number,
    cell: CellRef,
): void {
    pendingFocus.set(view, { tableFrom, ...cell });
}

export function applyTableModel(
    view: EditorView,
    from: number,
    contentTo: number,
    model: TableModel,
    resume?: CellRef,
): void {
    const insert = serializeTable(model);
    if (resume) queueTableCellFocus(view, from, resume);
    view.dispatch({
        changes: { from, to: contentTo, insert },
        userEvent: 'input',
    });
}

export function focusAfterTable(view: EditorView, blockTo: number): void {
    const br = view.state.lineBreak;
    let docLen = view.state.doc.length;
    const changes = blockTo >= docLen
        ? [{ from: docLen, insert: br }] as const
        : [];

    if (changes.length) docLen += br.length;

    view.dispatch({
        changes,
        selection: EditorSelection.cursor(Math.min(blockTo, docLen)),
        scrollIntoView: true,
    });
    view.focus();
}

export function insertTable(view: EditorView): boolean {
    if (view.state.readOnly) return false;

    const { from, to } = view.state.selection.main;
    const doc = view.state.doc;
    const line = doc.lineAt(from);
    const tableMd = serializeTable(createEmptyTableModel(2, 1));

    let prefix = '';
    if (from > line.from) prefix = '\n';
    else if (from > 0 && doc.sliceString(from - 1, from) !== '\n') prefix = '\n';

    const text = `${prefix}${tableMd}\n`;
    const tableFrom = from + prefix.length;

    queueTableCellFocus(view, tableFrom, { row: 0, col: 0 });
    view.dispatch({
        changes: { from, to, insert: text },
        selection: EditorSelection.cursor(tableFrom + tableMd.length + 1),
        userEvent: 'input',
        scrollIntoView: true,
    });
    view.focus();
    return true;
}
