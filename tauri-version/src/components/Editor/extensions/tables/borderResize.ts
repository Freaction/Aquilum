import {
    MIN_COL_WIDTH,
    MIN_ROW_HEIGHT,
    setRowHeight,
    withExplicitColWidths,
    type TableModel,
} from './model';
import { measureColWidths, measureRowHeights, tableOverlayHost } from './geometry';

const BORDER_HIT_PX = 6;

type BorderResizeAxis = 'row' | 'col';

interface BorderHit {
    axis: BorderResizeAxis;
    index: number;
    line: number;
    spanStart: number;
    spanEnd: number;
}

export interface BorderResizeState {
    axis: BorderResizeAxis;
    index: number;
    startPos: number;
    startSizes: number[];
}

function layoutBounds(root: HTMLElement): DOMRect | null {
    return root.querySelector('.q-md-table-layout')?.getBoundingClientRect() ?? null;
}

function rowBorderYs(root: HTMLElement): number[] {
    return [...root.querySelectorAll<HTMLElement>('.q-md-table-row-strip-btn')]
        .map((button) => button.getBoundingClientRect().bottom);
}

function colBorderXs(root: HTMLElement): number[] {
    return [...root.querySelectorAll<HTMLElement>('.q-md-table-col-strip-btn')]
        .map((button) => button.getBoundingClientRect().right);
}

export function hitTestBorder(root: HTMLElement, clientX: number, clientY: number): BorderHit | null {
    const bounds = layoutBounds(root);
    if (!bounds) return null;

    const pad = BORDER_HIT_PX;
    if (
        clientX < bounds.left - pad
        || clientX > bounds.right + pad
        || clientY < bounds.top - pad
        || clientY > bounds.bottom + pad
    ) {
        return null;
    }

    let best: BorderHit | null = null;
    let bestDist = Number.POSITIVE_INFINITY;

    for (const [index, line] of rowBorderYs(root).entries()) {
        const dist = Math.abs(clientY - line);
        if (dist > BORDER_HIT_PX || dist >= bestDist) continue;
        bestDist = dist;
        best = { axis: 'row', index, line, spanStart: bounds.left, spanEnd: bounds.right };
    }

    for (const [index, line] of colBorderXs(root).entries()) {
        const dist = Math.abs(clientX - line);
        if (dist > BORDER_HIT_PX || dist >= bestDist) continue;
        bestDist = dist;
        best = { axis: 'col', index, line, spanStart: bounds.top, spanEnd: bounds.bottom };
    }

    return best;
}

export function beginBorderResize(
    hit: BorderHit,
    root: HTMLElement,
    model: TableModel,
    event: MouseEvent,
): BorderResizeState {
    return {
        axis: hit.axis,
        index: hit.index,
        startPos: hit.axis === 'row' ? event.clientY : event.clientX,
        startSizes: hit.axis === 'row'
            ? measureRowHeights(root, model)
            : measureColWidths(root, model),
    };
}

export function applyBorderResize(
    model: TableModel,
    state: BorderResizeState,
    event: MouseEvent,
): TableModel | null {
    const startSize = state.startSizes[state.index];
    if (startSize == null) return null;
    const delta = state.axis === 'row'
        ? event.clientY - state.startPos
        : event.clientX - state.startPos;

    if (state.axis === 'row') {
        return setRowHeight(model, state.index, Math.max(MIN_ROW_HEIGHT, startSize + delta));
    }

    const widths = state.startSizes.slice();
    widths[state.index] = Math.max(MIN_COL_WIDTH, startSize + delta);
    return withExplicitColWidths(model, widths);
}

function ensureResizeLine(root: HTMLElement): HTMLElement {
    let line = root.querySelector('.q-md-table-resize-line') as HTMLElement | null;
    if (!line) {
        line = document.createElement('div');
        line.className = 'q-md-table-resize-line';
        line.hidden = true;
    }
    const host = tableOverlayHost(root);
    if (line.parentElement !== host) host.appendChild(line);
    return line;
}

export function hideResizeLine(root: HTMLElement): void {
    const line = root.querySelector('.q-md-table-resize-line') as HTMLElement | null;
    if (line) line.hidden = true;
    root.classList.remove('q-md-table-border-resizing-row', 'q-md-table-border-resizing-col');
}

export function paintResizeLine(root: HTMLElement, hit: BorderHit): void {
    const line = ensureResizeLine(root);
    const hostRect = tableOverlayHost(root).getBoundingClientRect();
    line.hidden = false;
    line.dataset.axis = hit.axis;
    if (hit.axis === 'row') {
        line.style.top = `${hit.line - hostRect.top - 1}px`;
        line.style.left = `${hit.spanStart - hostRect.left}px`;
        line.style.width = `${Math.max(0, hit.spanEnd - hit.spanStart)}px`;
        line.style.height = '';
        root.classList.add('q-md-table-border-resizing-row');
        root.classList.remove('q-md-table-border-resizing-col');
    } else {
        line.style.left = `${hit.line - hostRect.left - 1}px`;
        line.style.top = `${hit.spanStart - hostRect.top}px`;
        line.style.height = `${Math.max(0, hit.spanEnd - hit.spanStart)}px`;
        line.style.width = '';
        root.classList.add('q-md-table-border-resizing-col');
        root.classList.remove('q-md-table-border-resizing-row');
    }
}

export function paintResizeLineAtIndex(
    root: HTMLElement,
    axis: BorderResizeAxis,
    index: number,
): void {
    const bounds = layoutBounds(root);
    if (!bounds) return;
    if (axis === 'row') {
        const line = rowBorderYs(root)[index];
        if (line == null) return;
        paintResizeLine(root, {
            axis,
            index,
            line,
            spanStart: bounds.left,
            spanEnd: bounds.right,
        });
        return;
    }
    const line = colBorderXs(root)[index];
    if (line == null) return;
    paintResizeLine(root, {
        axis,
        index,
        line,
        spanStart: bounds.top,
        spanEnd: bounds.bottom,
    });
}
