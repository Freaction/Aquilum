import type { CellRef } from './model';
import type { CellNavAction } from './cellEditor';
import {
    anchorAt,
    columnCount,
    insertRow,
    mergeAt,
    rowCount,
    type TableModel,
} from './model';
import type { WidgetDocFlush } from './widgetSessionDoc';

export interface CellNavigationHost {
    getModel(): TableModel;
    setModel(model: TableModel): void;
    commitCell(cell: CellRef, value: string): { prev: string; next: string };
    docFlush: WidgetDocFlush;
    leaveTable(): void;
    applyStructure(resume: CellRef): void;
    openCellEditor(cell: CellRef): void;
}

function visibleCells(model: TableModel): CellRef[] {
    const cells: CellRef[] = [];
    for (let row = 0; row < rowCount(model); row += 1) {
        for (let col = 0; col < columnCount(model); col += 1) {
            const anchor = anchorAt(model, row, col);
            if (anchor?.row === row && anchor.col === col) cells.push(anchor);
        }
    }
    return cells;
}

function navigate(model: TableModel, cell: CellRef, action: CellNavAction): CellRef {
    const current = anchorAt(model, cell.row, cell.col) ?? cell;
    if (action === 'down') {
        const row = (mergeAt(model, current.row, current.col)?.bottom ?? current.row) + 1;
        return anchorAt(model, row, current.col) ?? current;
    }

    const cells = visibleCells(model);
    const index = cells.findIndex((candidate) => (
        candidate.row === current.row && candidate.col === current.col
    ));
    if (index < 0) return current;
    if (action === 'next') return cells[index + 1] ?? current;
    if (action === 'prev') return cells[index - 1] ?? current;
    return current;
}

export function handleCellNavigation(
    host: CellNavigationHost,
    cell: CellRef,
    action: CellNavAction,
    value: string,
): void {
    const { prev, next } = host.commitCell(cell, value);

    if (action === 'escape') {
        host.leaveTable();
        return;
    }

    if (action === 'blur') {
        if (next !== prev) host.docFlush.markDirty();
        return;
    }

    const rows = rowCount(host.getModel());
    const current = anchorAt(host.getModel(), cell.row, cell.col) ?? cell;
    let target = navigate(host.getModel(), current, action);
    const stayed = target.row === current.row && target.col === current.col;
    let structural = false;

    if (action === 'down' && stayed) {
        host.setModel(insertRow(host.getModel(), rows));
        target = { row: rows, col: current.col };
        structural = true;
    } else if (action === 'next' && stayed) {
        host.setModel(insertRow(host.getModel(), rows));
        target = { row: rows, col: 0 };
        structural = true;
    }

    if (next !== prev) host.docFlush.markDirty();

    if (structural) {
        host.applyStructure(target);
        return;
    }

    host.openCellEditor(target);
}
