import { EditorSelection } from '@codemirror/state';
import type { EditorView } from '@codemirror/view';
import { applyTableModel } from './apply';
import { findTablesInDoc } from './constructs';
import {
    columnCount,
    deleteColumn,
    deleteRow,
    insertColumn,
    insertRow,
    rowCount,
    type TableModel,
    type CellRef,
} from './model';

const applyGeneration = new WeakMap<EditorView, number>();

function nextApplyGeneration(view: EditorView): number {
    const id = (applyGeneration.get(view) ?? 0) + 1;
    applyGeneration.set(view, id);
    return id;
}

export function scheduleApplyTableModel(
    view: EditorView,
    from: number,
    contentTo: number,
    model: TableModel,
    resume?: CellRef,
): void {
    const generation = nextApplyGeneration(view);
    queueMicrotask(() => {
        if (!view.dom.isConnected) return;
        if (applyGeneration.get(view) !== generation) return;
        const live = findTablesInDoc(view.state.doc).find((table) => table.from === from);
        const applyFrom = live?.from ?? from;
        const applyTo = live?.contentTo ?? contentTo;
        applyTableModel(
            view,
            applyFrom,
            applyTo,
            model,
            resume,
        );
    });
}

export function addTableRow(model: TableModel): { model: TableModel; resume: CellRef } {
    const row = rowCount(model);
    return { model: insertRow(model, row), resume: { row, col: 0 } };
}

export function addTableColumn(model: TableModel): { model: TableModel; resume: CellRef } {
    const col = columnCount(model);
    return { model: insertColumn(model, col), resume: { row: 0, col } };
}

export function removeTableRow(
    model: TableModel,
    row: number,
    col: number,
): { model: TableModel; resume: CellRef } | null {
    const next = deleteRow(model, row);
    if (!next) return null;
    return {
        model: next,
        resume: { row: Math.min(row, rowCount(next) - 1), col },
    };
}

export function removeTableColumn(
    model: TableModel,
    row: number,
    col: number,
): { model: TableModel; resume: CellRef } | null {
    const next = deleteColumn(model, col);
    if (!next) return null;
    return {
        model: next,
        resume: { row, col: Math.min(col, columnCount(next) - 1) },
    };
}

export function scheduleDeleteTable(
    view: EditorView,
    from: number,
    blockTo: number,
): void {
    nextApplyGeneration(view);
    queueMicrotask(() => {
        if (!view.dom.isConnected) return;
        view.dispatch({
            changes: { from, to: blockTo, insert: '' },
            selection: EditorSelection.cursor(from),
            userEvent: 'delete',
            scrollIntoView: true,
        });
        view.focus();
    });
}
