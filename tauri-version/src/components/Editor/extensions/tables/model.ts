export type TableAlign = 'left' | 'center' | 'right' | null;

export interface CellRef {
    row: number;
    col: number;
}

export interface MergeRange {
    top: number;
    left: number;
    bottom: number;
    right: number;
}

type TableColumnSizing = 'auto' | 'explicit';

export interface TableModel {
    cells: string[][];
    merges: MergeRange[];
    aligns: TableAlign[];
    rowHeights: number[];
    colWidths: number[];
    colSizing: TableColumnSizing;
}

export const DEFAULT_ROW_HEIGHT = 40;
export const MIN_ROW_HEIGHT = 28;
export const DEFAULT_COL_WIDTH = 60;
export const MIN_COL_WIDTH = 36;

function normalizeRowHeight(height: number | undefined): number {
    return Number.isFinite(height) ? Math.max(MIN_ROW_HEIGHT, height as number) : DEFAULT_ROW_HEIGHT;
}

function normalizeColWidth(width: number | undefined): number {
    return Number.isFinite(width) ? Math.max(MIN_COL_WIDTH, width as number) : DEFAULT_COL_WIDTH;
}

export function rowCount(model: TableModel): number {
    return model.cells.length;
}

export function columnCount(model: TableModel): number {
    return model.aligns.length;
}

export function createEmptyTableModel(cols: number, bodyRows: number): TableModel {
    const columns = Math.max(1, Math.floor(cols));
    const rows = Math.max(1, Math.floor(bodyRows)) + 1;
    return {
        cells: Array.from({ length: rows }, () => Array.from({ length: columns }, () => '')),
        merges: [],
        aligns: Array.from({ length: columns }, () => null),
        rowHeights: Array.from({ length: rows }, () => DEFAULT_ROW_HEIGHT),
        colWidths: Array.from({ length: columns }, () => DEFAULT_COL_WIDTH),
        colSizing: 'auto',
    };
}

export function normalizeTableModel(model: TableModel): TableModel {
    const cols = columnCount(model);
    const cells = model.cells.map((row) => Array.from({ length: cols }, (_, col) => row[col] ?? ''));
    const rows = cells.length;
    const merges: MergeRange[] = [];
    for (const merge of model.merges) {
        if (![merge.top, merge.left, merge.bottom, merge.right].every(Number.isInteger)) continue;
        const range = {
            top: Math.max(0, merge.top),
            left: Math.max(0, merge.left),
            bottom: Math.min(rows - 1, merge.bottom),
            right: Math.min(cols - 1, merge.right),
        };
        if (range.top > range.bottom || range.left > range.right) continue;
        if (range.top === range.bottom && range.left === range.right) continue;
        if (merges.some((other) => rangesIntersect(other, range))) continue;
        merges.push(range);
    }
    merges.sort((a, b) => a.top - b.top || a.left - b.left);
    return synchronizeMergeValues({
        cells,
        merges,
        aligns: model.aligns.slice(),
        rowHeights: Array.from({ length: rows }, (_, row) => normalizeRowHeight(model.rowHeights?.[row])),
        colWidths: Array.from({ length: cols }, (_, col) => normalizeColWidth(model.colWidths?.[col])),
        colSizing: model.colSizing ?? 'auto',
    });
}

function synchronizeMergeValues(model: TableModel): TableModel {
    const cells = model.cells.map((row) => row.slice());
    for (const merge of model.merges) {
        const value = cells[merge.top]?.[merge.left] ?? '';
        for (let row = merge.top; row <= merge.bottom; row += 1) {
            for (let col = merge.left; col <= merge.right; col += 1) cells[row][col] = value;
        }
    }
    return { ...model, cells };
}

export function rangesIntersect(a: MergeRange, b: MergeRange): boolean {
    return a.top <= b.bottom && a.bottom >= b.top && a.left <= b.right && a.right >= b.left;
}

export function rangeContains(range: MergeRange, row: number, col: number): boolean {
    return row >= range.top && row <= range.bottom && col >= range.left && col <= range.right;
}

function rangeContainsRange(outer: MergeRange, inner: MergeRange): boolean {
    return outer.top <= inner.top
        && outer.left <= inner.left
        && outer.bottom >= inner.bottom
        && outer.right >= inner.right;
}

export function mergeAt(model: TableModel, row: number, col: number): MergeRange | null {
    return model.merges.find((merge) => rangeContains(merge, row, col)) ?? null;
}

export function anchorAt(model: TableModel, row: number, col: number): CellRef | null {
    if (row < 0 || col < 0 || row >= rowCount(model) || col >= columnCount(model)) return null;
    const merge = mergeAt(model, row, col);
    return merge ? { row: merge.top, col: merge.left } : { row, col };
}

export function cellValue(model: TableModel, row: number, col: number): string {
    const anchor = anchorAt(model, row, col);
    return anchor ? model.cells[anchor.row][anchor.col] : '';
}

export function cellIsMerged(model: TableModel, row: number, col: number): boolean {
    return mergeAt(model, row, col) !== null;
}

export function setCell(model: TableModel, row: number, col: number, value: string): TableModel {
    const anchor = anchorAt(model, row, col);
    if (!anchor) return model;
    const cells = model.cells.map((current) => current.slice());
    const merge = mergeAt(model, anchor.row, anchor.col);
    if (!merge) {
        cells[anchor.row][anchor.col] = value;
    } else {
        for (let r = merge.top; r <= merge.bottom; r += 1) {
            for (let c = merge.left; c <= merge.right; c += 1) cells[r][c] = value;
        }
    }
    return { ...model, cells };
}

export function setColumnAlign(model: TableModel, col: number, align: TableAlign): TableModel | null {
    if (col < 0 || col >= columnCount(model)) return null;
    const aligns = model.aligns.slice();
    aligns[col] = align;
    return { ...model, aligns };
}

export function columnAlign(model: TableModel, col: number): TableAlign {
    return model.aligns[col] ?? null;
}

export function setRowHeight(model: TableModel, row: number, height: number): TableModel | null {
    if (row < 0 || row >= rowCount(model)) return null;
    const rowHeights = model.rowHeights.slice();
    rowHeights[row] = normalizeRowHeight(height);
    return { ...model, rowHeights };
}

export function withExplicitColWidths(
    model: TableModel,
    widths: readonly number[],
): TableModel {
    return {
        ...model,
        colWidths: Array.from(
            { length: columnCount(model) },
            (_, col) => normalizeColWidth(widths[col] ?? model.colWidths[col]),
        ),
        colSizing: 'explicit',
    };
}

export function totalColWidth(model: TableModel): number {
    return model.colWidths.reduce((sum, width) => sum + width, 0);
}

export function mergeCells(model: TableModel, refs: CellRef[]): TableModel | null {
    if (refs.length < 2) return null;
    const unique = new Set(refs.map((ref) => `${ref.row}:${ref.col}`));
    if (unique.size !== refs.length) return null;
    if (refs.some((ref) => ref.row < 0 || ref.col < 0 || ref.row >= rowCount(model) || ref.col >= columnCount(model))) return null;
    const top = Math.min(...refs.map((ref) => ref.row));
    const bottom = Math.max(...refs.map((ref) => ref.row));
    const left = Math.min(...refs.map((ref) => ref.col));
    const right = Math.max(...refs.map((ref) => ref.col));
    if (refs.length !== (bottom - top + 1) * (right - left + 1)) return null;

    const range = { top, left, bottom, right };
    if (model.merges.some((merge) => (
        rangesIntersect(range, merge) && !rangeContainsRange(range, merge)
    ))) return null;
    if (model.merges.some((merge) => merge.top === range.top && merge.left === range.left && merge.bottom === range.bottom && merge.right === range.right)) return null;

    const cells = model.cells.map((row) => row.slice());
    const value = cellValue(model, range.top, range.left);
    for (let row = range.top; row <= range.bottom; row += 1) {
        for (let col = range.left; col <= range.right; col += 1) cells[row][col] = value;
    }
    return {
        ...model,
        cells,
        merges: [...model.merges.filter((merge) => !rangesIntersect(merge, range)), range]
            .sort((a, b) => a.top - b.top || a.left - b.left),
    };
}

export function unmergeCell(model: TableModel, row: number, col: number): TableModel | null {
    const merge = mergeAt(model, row, col);
    if (!merge) return null;
    const cells = model.cells.map((current) => current.slice());
    const value = cells[merge.top][merge.left];
    for (let r = merge.top; r <= merge.bottom; r += 1) {
        for (let c = merge.left; c <= merge.right; c += 1) cells[r][c] = r === merge.top && c === merge.left ? value : '';
    }
    return { ...model, cells, merges: model.merges.filter((current) => current !== merge) };
}

export function clearCells(model: TableModel, refs: CellRef[]): TableModel {
    const cells = model.cells.map((row) => row.slice());
    const seen = new Set<string>();
    for (const ref of refs) {
        const anchor = anchorAt(model, ref.row, ref.col);
        if (!anchor) continue;
        const key = `${anchor.row}:${anchor.col}`;
        if (seen.has(key)) continue;
        seen.add(key);
        const merge = mergeAt(model, anchor.row, anchor.col);
        if (!merge) {
            cells[anchor.row][anchor.col] = '';
            continue;
        }
        for (let row = merge.top; row <= merge.bottom; row += 1) {
            for (let col = merge.left; col <= merge.right; col += 1) cells[row][col] = '';
        }
    }
    return { ...model, cells };
}

export function insertRow(model: TableModel, index: number): TableModel {
    const at = Math.max(0, Math.min(index, rowCount(model)));
    const cells = model.cells.map((row) => row.slice());
    cells.splice(at, 0, Array.from({ length: columnCount(model) }, () => ''));
    const rowHeights = model.rowHeights.slice();
    rowHeights.splice(at, 0, DEFAULT_ROW_HEIGHT);
    const merges = model.merges.map((merge) => ({
        ...merge,
        top: merge.top >= at ? merge.top + 1 : merge.top,
        bottom: merge.bottom >= at ? merge.bottom + 1 : merge.bottom,
    }));
    const next = { ...model, cells, rowHeights, merges };
    for (let mergeIndex = 0; mergeIndex < merges.length; mergeIndex += 1) {
        const source = model.merges[mergeIndex];
        const target = merges[mergeIndex];
        cells[target.top][target.left] = cellValue(model, source.top, source.left);
    }
    return synchronizeMergeValues(next);
}

export function deleteRow(model: TableModel, index: number): TableModel | null {
    if (index < 0 || index >= rowCount(model) || rowCount(model) <= 2) return null;
    const cells = model.cells.filter((_, row) => row !== index).map((row) => row.slice());
    const rowHeights = model.rowHeights.filter((_, row) => row !== index);
    const transformed = model.merges.flatMap((merge) => {
        let range: MergeRange | null = merge;
        if (index < merge.top) range = { ...merge, top: merge.top - 1, bottom: merge.bottom - 1 };
        else if (index <= merge.bottom) range = merge.top === merge.bottom ? null : { ...merge, bottom: merge.bottom - 1 };
        if (!range || (range.top === range.bottom && range.left === range.right)) return [];
        return [{ range, value: cellValue(model, merge.top, merge.left) }];
    });
    for (const item of transformed) cells[item.range.top][item.range.left] = item.value;
    return synchronizeMergeValues({ ...model, cells, rowHeights, merges: transformed.map((item) => item.range) });
}

export function insertColumn(model: TableModel, index: number): TableModel {
    const at = Math.max(0, Math.min(index, columnCount(model)));
    const cells = model.cells.map((row) => [...row.slice(0, at), '', ...row.slice(at)]);
    const aligns = [...model.aligns.slice(0, at), null, ...model.aligns.slice(at)];
    const colWidths = [...model.colWidths.slice(0, at), DEFAULT_COL_WIDTH, ...model.colWidths.slice(at)];
    const merges = model.merges.map((merge) => ({
        ...merge,
        left: merge.left >= at ? merge.left + 1 : merge.left,
        right: merge.right >= at ? merge.right + 1 : merge.right,
    }));
    for (let mergeIndex = 0; mergeIndex < merges.length; mergeIndex += 1) {
        const source = model.merges[mergeIndex];
        const target = merges[mergeIndex];
        cells[target.top][target.left] = cellValue(model, source.top, source.left);
    }
    return synchronizeMergeValues({ ...model, cells, aligns, colWidths, merges });
}

export function deleteColumn(model: TableModel, index: number): TableModel | null {
    if (index < 0 || index >= columnCount(model) || columnCount(model) <= 1) return null;
    const cells = model.cells.map((row) => row.filter((_, col) => col !== index));
    const aligns = model.aligns.filter((_, col) => col !== index);
    const colWidths = model.colWidths.filter((_, col) => col !== index);
    const transformed = model.merges.flatMap((merge) => {
        let range: MergeRange | null = merge;
        if (index < merge.left) range = { ...merge, left: merge.left - 1, right: merge.right - 1 };
        else if (index <= merge.right) range = merge.left === merge.right ? null : { ...merge, right: merge.right - 1 };
        if (!range || (range.top === range.bottom && range.left === range.right)) return [];
        return [{ range, value: cellValue(model, merge.top, merge.left) }];
    });
    for (const item of transformed) cells[item.range.top][item.range.left] = item.value;
    return synchronizeMergeValues({ ...model, cells, aligns, colWidths, merges: transformed.map((item) => item.range) });
}

export function destFromInsertGap(start: number, end: number, insertGap: number): number {
    const size = end - start + 1;
    if (insertGap <= start) return insertGap;
    if (insertGap > end) return insertGap - size;
    return start;
}

export function moveRows(
    model: TableModel,
    top: number,
    bottom: number,
    dest: number,
): TableModel | null {
    const rows = rowCount(model);
    if (top < 0 || bottom >= rows || top > bottom) return null;
    const size = bottom - top + 1;
    const clamped = Math.max(0, Math.min(dest, rows - size));
    if (clamped === top) return model;

    const order = Array.from({ length: rows }, (_, index) => index);
    const block = order.splice(top, size);
    order.splice(clamped, 0, ...block);

    const oldToNew = Array.from({ length: rows }, () => 0);
    order.forEach((oldIndex, newIndex) => {
        oldToNew[oldIndex] = newIndex;
    });

    return synchronizeMergeValues(normalizeTableModel({
        cells: order.map((oldIndex) => model.cells[oldIndex].slice()),
        rowHeights: order.map((oldIndex) => model.rowHeights[oldIndex]),
        colWidths: model.colWidths.slice(),
        colSizing: model.colSizing,
        aligns: model.aligns.slice(),
        merges: model.merges.map((merge) => ({
            top: oldToNew[merge.top],
            bottom: oldToNew[merge.bottom],
            left: merge.left,
            right: merge.right,
        })),
    }));
}

export function moveColumns(
    model: TableModel,
    left: number,
    right: number,
    dest: number,
): TableModel | null {
    const cols = columnCount(model);
    if (left < 0 || right >= cols || left > right) return null;
    const size = right - left + 1;
    const clamped = Math.max(0, Math.min(dest, cols - size));
    if (clamped === left) return model;

    const order = Array.from({ length: cols }, (_, index) => index);
    const block = order.splice(left, size);
    order.splice(clamped, 0, ...block);

    const oldToNew = Array.from({ length: cols }, () => 0);
    order.forEach((oldIndex, newIndex) => {
        oldToNew[oldIndex] = newIndex;
    });

    return synchronizeMergeValues(normalizeTableModel({
        cells: model.cells.map((row) => order.map((oldIndex) => row[oldIndex])),
        rowHeights: model.rowHeights.slice(),
        colWidths: order.map((oldIndex) => model.colWidths[oldIndex]),
        colSizing: model.colSizing,
        aligns: order.map((oldIndex) => model.aligns[oldIndex]),
        merges: model.merges.map((merge) => ({
            top: merge.top,
            bottom: merge.bottom,
            left: oldToNew[merge.left],
            right: oldToNew[merge.right],
        })),
    }));
}

