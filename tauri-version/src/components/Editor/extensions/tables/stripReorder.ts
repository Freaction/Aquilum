import {
    columnCount,
    destFromInsertGap,
    moveColumns,
    moveRows,
    rowCount,
    type CellRef,
    type TableModel,
} from './model';
import { expandSelection, normalizeRect } from './selection';
import { tableOverlayHost } from './geometry';

type StripDragAxis = 'row' | 'col';

export interface StripDragState {
    axis: StripDragAxis;
    start: number;
    end: number;
    startX: number;
    startY: number;
    dragging: boolean;
    insertGap: number;
    gapValid: boolean;
}

const DRAG_THRESHOLD_PX = 4;

export function expandedRowBlock(model: TableModel, row: number): { top: number; bottom: number } {
    const selection = expandSelection(model, {
        anchor: { row, col: 0 },
        focus: { row, col: columnCount(model) - 1 },
    });
    const rect = normalizeRect(selection.anchor, selection.focus);
    return { top: rect.top, bottom: rect.bottom };
}

export function expandedColBlock(model: TableModel, col: number): { left: number; right: number } {
    const selection = expandSelection(model, {
        anchor: { row: 0, col },
        focus: { row: rowCount(model) - 1, col },
    });
    const rect = normalizeRect(selection.anchor, selection.focus);
    return { left: rect.left, right: rect.right };
}

export function isValidInsertGap(
    model: TableModel,
    axis: StripDragAxis,
    start: number,
    end: number,
    gap: number,
): boolean {
    if (destFromInsertGap(start, end, gap) === start) return false;

    for (const merge of model.merges) {
        if (axis === 'col') {
            if (merge.left >= start && merge.right <= end) continue;
            if (merge.left < gap && gap <= merge.right) return false;
        } else {
            if (merge.top >= start && merge.bottom <= end) continue;
            if (merge.top < gap && gap <= merge.bottom) return false;
        }
    }
    return true;
}

export function beginStripDrag(
    axis: StripDragAxis,
    start: number,
    end: number,
    event: MouseEvent,
): StripDragState {
    return {
        axis,
        start,
        end,
        startX: event.clientX,
        startY: event.clientY,
        dragging: false,
        insertGap: start,
        gapValid: false,
    };
}

export function updateStripDrag(
    state: StripDragState,
    model: TableModel,
    root: HTMLElement,
    event: MouseEvent,
): void {
    if (!state.dragging) {
        if (Math.hypot(event.clientX - state.startX, event.clientY - state.startY) < DRAG_THRESHOLD_PX) {
            return;
        }
        state.dragging = true;
    }

    const rawGap = insertGapFromPointer(root, event, state.axis);
    const gap = snapInsertGap(model, state.axis, state.start, state.end, rawGap);
    state.insertGap = gap;
    state.gapValid = isValidInsertGap(model, state.axis, state.start, state.end, gap);
    paintStripDropLine(root, state);
}

export function clearStripDrag(root: HTMLElement): void {
    hideStripDropLine(root);
}

export function commitStripDrag(
    model: TableModel,
    state: StripDragState,
): { model: TableModel; resume: CellRef } | null {
    if (!state.gapValid) return null;
    const dest = destFromInsertGap(state.start, state.end, state.insertGap);
    if (dest === state.start) return null;

    if (state.axis === 'row') {
        const next = moveRows(model, state.start, state.end, dest);
        if (!next || next === model) return null;
        return { model: next, resume: { row: dest, col: 0 } };
    }

    const next = moveColumns(model, state.start, state.end, dest);
    if (!next || next === model) return null;
    return { model: next, resume: { row: 0, col: dest } };
}

function snapInsertGap(
    model: TableModel,
    axis: StripDragAxis,
    start: number,
    end: number,
    gap: number,
): number {
    if (isValidInsertGap(model, axis, start, end, gap)) return gap;

    const maxGap = axis === 'row' ? rowCount(model) : columnCount(model);
    for (let distance = 1; distance <= maxGap; distance += 1) {
        const left = gap - distance;
        if (left >= 0 && isValidInsertGap(model, axis, start, end, left)) return left;
        const right = gap + distance;
        if (right <= maxGap && isValidInsertGap(model, axis, start, end, right)) return right;
    }
    return gap;
}

function insertGapFromPointer(root: HTMLElement, event: MouseEvent, axis: StripDragAxis): number {
    const selector = axis === 'row' ? '.q-md-table-row-strip-btn' : '.q-md-table-col-strip-btn';
    const pointer = axis === 'row' ? event.clientY : event.clientX;
    const buttons = [...root.querySelectorAll<HTMLElement>(selector)];
    if (buttons.length === 0) return 0;
    const mids = buttons.map((button) => {
        const rect = button.getBoundingClientRect();
        return axis === 'row' ? (rect.top + rect.bottom) / 2 : (rect.left + rect.right) / 2;
    });
    if (pointer < mids[0]) return 0;
    for (let index = 0; index < mids.length - 1; index += 1) {
        if (pointer < (mids[index] + mids[index + 1]) / 2) return index + 1;
    }
    return buttons.length;
}

function ensureDropLine(root: HTMLElement): HTMLElement {
    let line = root.querySelector('.q-md-table-strip-drop') as HTMLElement | null;
    if (!line) {
        line = document.createElement('div');
        line.className = 'q-md-table-strip-drop';
        line.hidden = true;
    }
    const host = tableOverlayHost(root);
    if (line.parentElement !== host) host.appendChild(line);
    return line;
}

function hideStripDropLine(root: HTMLElement): void {
    const line = root.querySelector('.q-md-table-strip-drop') as HTMLElement | null;
    if (line) line.hidden = true;
}

function paintStripDropLine(root: HTMLElement, state: StripDragState): void {
    const line = ensureDropLine(root);
    const host = tableOverlayHost(root);
    const hostRect = host.getBoundingClientRect();
    line.dataset.valid = state.gapValid ? 'true' : 'false';

    if (state.axis === 'row') {
        const buttons = [...root.querySelectorAll<HTMLElement>('.q-md-table-row-strip-btn')];
        if (buttons.length === 0) {
            line.hidden = true;
            return;
        }
        const y = state.insertGap >= buttons.length
            ? buttons[buttons.length - 1].getBoundingClientRect().bottom
            : buttons[state.insertGap].getBoundingClientRect().top;
        line.hidden = false;
        line.dataset.axis = 'row';
        line.style.top = `${y - hostRect.top - 1}px`;
        line.style.left = '0px';
        line.style.width = `${hostRect.width}px`;
        line.style.height = '';
        return;
    }

    const buttons = [...root.querySelectorAll<HTMLElement>('.q-md-table-col-strip-btn')];
    if (buttons.length === 0) {
        line.hidden = true;
        return;
    }
    const x = state.insertGap >= buttons.length
        ? buttons[buttons.length - 1].getBoundingClientRect().right
        : buttons[state.insertGap].getBoundingClientRect().left;
    const body = root.querySelector('.q-md-table-body') as HTMLElement | null;
    const bodyRect = (body ?? host).getBoundingClientRect();
    line.hidden = false;
    line.dataset.axis = 'col';
    line.style.left = `${x - hostRect.left - 1}px`;
    line.style.top = '0px';
    line.style.height = `${bodyRect.bottom - hostRect.top}px`;
    line.style.width = '';
}
