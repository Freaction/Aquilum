import type { EditorView } from '@codemirror/view';
import {
    clearCells,
    columnCount,
    mergeCells,
    rangesIntersect,
    rowCount,
    type CellRef,
    type MergeRange,
    type TableModel,
} from './model';
import { collapseSelection } from '../editorFocus';
import { setTableInteractionChrome } from './parentSelection';
import { paintTableSelection } from './widgetDom';

export interface TableSelection {
    anchor: CellRef;
    focus: CellRef;
}

export interface TableSelectionHost {
    readonly view: EditorView;
    readonly root: HTMLElement;
    getModel(): TableModel;
    setModel(model: TableModel): void;
    absorbActiveCell(): void;
    markDocDirty(): void;
    repaintModel(): void;
    getSelection(): TableSelection | null;
    setSelection(selection: TableSelection | null): void;
}

export interface TablePointerState {
    anchor: CellRef | null;
    startX: number;
    startY: number;
    dragging: boolean;
}

export function normalizeRect(a: CellRef, b: CellRef): MergeRange {
    return {
        top: Math.min(a.row, b.row),
        left: Math.min(a.col, b.col),
        bottom: Math.max(a.row, b.row),
        right: Math.max(a.col, b.col),
    };
}

function cellsForSelection(selection: TableSelection): CellRef[] {
    const range = normalizeRect(selection.anchor, selection.focus);
    const cells: CellRef[] = [];
    for (let row = range.top; row <= range.bottom; row += 1) {
        for (let col = range.left; col <= range.right; col += 1) cells.push({ row, col });
    }
    return cells;
}

export function expandSelection(model: TableModel, selection: TableSelection): TableSelection {
    const forwardRows = selection.anchor.row <= selection.focus.row;
    const forwardColumns = selection.anchor.col <= selection.focus.col;
    let { top, left, bottom, right } = normalizeRect(selection.anchor, selection.focus);
    let changed = true;
    while (changed) {
        changed = false;
        for (const merge of model.merges) {
            if (!rangesIntersect(merge, { top, left, bottom, right })) continue;
            const next = {
                top: Math.min(top, merge.top),
                left: Math.min(left, merge.left),
                bottom: Math.max(bottom, merge.bottom),
                right: Math.max(right, merge.right),
            };
            if (next.top === top && next.left === left && next.bottom === bottom && next.right === right) continue;
            ({ top, left, bottom, right } = next);
            changed = true;
        }
    }
    return {
        anchor: {
            row: forwardRows ? top : bottom,
            col: forwardColumns ? left : right,
        },
        focus: {
            row: forwardRows ? bottom : top,
            col: forwardColumns ? right : left,
        },
    };
}

function applySelection(host: TableSelectionHost, selection: TableSelection): void {
    host.absorbActiveCell();
    collapseSelection(host.view);
    host.setSelection(expandSelection(host.getModel(), selection));
}

export function selectCellRange(host: TableSelectionHost, cell: CellRef, extend: boolean): void {
    const current = host.getSelection();
    const anchor = extend && current ? current.anchor : cell;
    applySelection(host, { anchor, focus: cell });
}

export function selectRow(host: TableSelectionHost, row: number, extend: boolean): void {
    const selection = { anchor: { row, col: 0 }, focus: { row, col: columnCount(host.getModel()) - 1 } };
    const current = host.getSelection();
    applySelection(host, extend && current ? { anchor: current.anchor, focus: selection.focus } : selection);
}

export function selectColumn(host: TableSelectionHost, col: number, extend: boolean): void {
    const selection = { anchor: { row: 0, col }, focus: { row: rowCount(host.getModel()) - 1, col } };
    const current = host.getSelection();
    applySelection(host, extend && current ? { anchor: current.anchor, focus: selection.focus } : selection);
}

export function selectedCells(host: TableSelectionHost): CellRef[] {
    const selection = host.getSelection();
    return selection ? cellsForSelection(selection) : [];
}

export function canMergeSelection(host: TableSelectionHost): boolean {
    const cells = selectedCells(host);
    return cells.length > 1 && mergeCells(host.getModel(), cells) !== null;
}

export function mergeSelection(host: TableSelectionHost): boolean {
    const cells = selectedCells(host);
    if (cells.length < 2) return false;
    host.absorbActiveCell();
    const model = mergeCells(host.getModel(), cells);
    if (!model) return false;
    host.setModel(model);
    host.repaintModel();
    host.markDocDirty();
    return true;
}

export function clearSelectionCells(host: TableSelectionHost): boolean {
    const cells = selectedCells(host);
    if (!cells.length) return false;
    host.absorbActiveCell();
    host.setModel(clearCells(host.getModel(), cells));
    host.repaintModel();
    host.markDocDirty();
    return true;
}

export function clearTableSelection(host: TableSelectionHost): void {
    host.setSelection(null);
}

export function syncSelectionPaint(host: TableSelectionHost): void {
    const selection = host.getSelection();
    paintTableSelection(host.root, selection ? normalizeRect(selection.anchor, selection.focus) : null);
    setTableInteractionChrome(host.view, selection !== null);
}

export function createPointerState(): TablePointerState {
    return { anchor: null, startX: 0, startY: 0, dragging: false };
}

export function beginCellPointer(state: TablePointerState, cell: CellRef, event: MouseEvent): void {
    state.anchor = cell;
    state.startX = event.clientX;
    state.startY = event.clientY;
    state.dragging = false;
}

export function resetPointerState(state: TablePointerState): void {
    state.anchor = null;
    state.dragging = false;
}

export function updatePointerDrag(
    host: TableSelectionHost,
    state: TablePointerState,
    focus: CellRef | null,
    event: MouseEvent,
): boolean {
    if (!state.anchor || !focus) return false;
    if (!state.dragging) {
        if (Math.hypot(event.clientX - state.startX, event.clientY - state.startY) < 4) return false;
        if (focus.row === state.anchor.row && focus.col === state.anchor.col) return false;
        state.dragging = true;
    }
    applySelection(host, { anchor: state.anchor, focus });
    return true;
}
