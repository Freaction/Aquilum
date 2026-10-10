import {
    DEFAULT_COL_WIDTH,
    DEFAULT_ROW_HEIGHT,
    MIN_COL_WIDTH,
    MIN_ROW_HEIGHT,
    normalizeTableModel,
    rangesIntersect,
    type MergeRange,
    type TableAlign,
    type TableModel,
} from './model';
import { SEPARATOR_CELL, splitTableRow } from './rows';

const EMPTY_CELL = '<!--q-empty-->';
const ROWSPAN = '^^';

const ESCAPED_EMPTY_CELL = '&lt;!--q-empty--&gt;';
const ESCAPED_ROWSPAN = '\\^\\^';

const metadataPattern = /^<!--q-table:(\{.*\})-->$/;

interface TableMetadata {
    merges?: number[][];
    rows?: number[];
    cols?: number[];
}

interface ParsedMetadata {
    present: boolean;
    value: TableMetadata;
}

function normalizeRow(cells: string[], columns: number): string[] {
    return Array.from({ length: columns }, (_, col) => cells[col] ?? EMPTY_CELL);
}

function cellValueFromMarkdown(value: string): string {
    if (value === EMPTY_CELL || value === ROWSPAN) return '';
    if (value === ESCAPED_EMPTY_CELL) return EMPTY_CELL;
    if (value === ESCAPED_ROWSPAN) return ROWSPAN;
    return value;
}

function cellValueToMarkdown(value: string): string {
    if (value === '') return EMPTY_CELL;
    if (value === EMPTY_CELL) return ESCAPED_EMPTY_CELL;
    if (value === ROWSPAN) return ESCAPED_ROWSPAN;
    return value;
}

function alignFromCell(cell: string): TableAlign | null {
    const value = cell.trim();
    if (!SEPARATOR_CELL.test(value)) return null;
    if (value.startsWith(':') && value.endsWith(':')) return 'center';
    if (value.startsWith(':')) return 'left';
    if (value.endsWith(':')) return 'right';
    return null;
}

function parseMetadata(line: string | undefined): ParsedMetadata {
    const trimmed = line?.trim();
    if (!trimmed?.startsWith('<!--q-table:')) return { present: false, value: {} };
    const match = trimmed.match(metadataPattern);
    if (!match) return { present: true, value: {} };
    try {
        const parsed = JSON.parse(match[1]);
        const value = parsed && typeof parsed === 'object' && !Array.isArray(parsed)
            ? parsed as TableMetadata
            : {};
        return { present: true, value };
    } catch {
        return { present: true, value: {} };
    }
}

function mergesFromMetadata(metadata: TableMetadata, rows: number, columns: number): MergeRange[] | null {
    if (!Array.isArray(metadata.merges)) return null;
    const merges: MergeRange[] = [];
    for (const raw of metadata.merges) {
        if (!Array.isArray(raw) || raw.length !== 4) return null;
        const [top, left, bottom, right] = raw.map(Number);
        const merge = { top, left, bottom, right };
        if (!Object.values(merge).every(Number.isInteger)) return null;
        if (top < 0 || left < 0 || bottom >= rows || right >= columns) return null;
        if (top > bottom || left > right || (top === bottom && left === right)) return null;
        if (merges.some((current) => rangesIntersect(current, merge))) return null;
        merges.push(merge);
    }
    return merges;
}

function inferredMerges(rawRows: string[][], columns: number): MergeRange[] {
    const merges: MergeRange[] = [];
    for (let row = 0; row < rawRows.length; row += 1) {
        let col = 0;
        while (col < columns) {
            const value = rawRows[row][col];
            if (value === ROWSPAN || value === EMPTY_CELL) {
                col += 1;
                continue;
            }
            let right = col;
            while (right + 1 < columns && rawRows[row][right + 1] === '') right += 1;
            if (right > col) merges.push({ top: row, left: col, bottom: row, right });
            col = right + 1;
        }
    }

    for (let row = 1; row < rawRows.length; row += 1) {
        let col = 0;
        while (col < columns) {
            if (rawRows[row][col] !== ROWSPAN) {
                col += 1;
                continue;
            }
            let right = col;
            while (right + 1 < columns && rawRows[row][right + 1] === ROWSPAN) right += 1;
            const existing = merges.find((merge) =>
                merge.bottom === row - 1 && merge.left === col && merge.right === right,
            );
            if (existing) existing.bottom = row;
            else merges.push({ top: row - 1, left: col, bottom: row, right });
            col = right + 1;
        }
    }

    return merges
        .sort((a, b) => a.top - b.top || a.left - b.left)
        .filter((merge, index, all) => !all.slice(0, index).some((other) => rangesIntersect(other, merge)));
}

export function parseMarkdownTable(text: string): TableModel | null {
    const lines = text.replace(/\r\n?/g, '\n').split('\n');
    const header = splitTableRow(lines[0] ?? '');
    const separator = splitTableRow(lines[1] ?? '');
    if (!header || !separator || !separator.every((cell) => SEPARATOR_CELL.test(cell))) return null;
    const columns = Math.max(header.length, separator.length);
    if (!columns) return null;

    const rawRows = [normalizeRow(header, columns)];
    let line = 2;
    while (line < lines.length) {
        const row = splitTableRow(lines[line]);
        if (!row) break;
        rawRows.push(normalizeRow(row, columns));
        line += 1;
    }
    if (rawRows.length < 2) return null;

    const parsedMetadata = parseMetadata(lines[line]);
    const metadataMerges = mergesFromMetadata(parsedMetadata.value, rawRows.length, columns);
    const merges = parsedMetadata.present
        ? metadataMerges ?? []
        : inferredMerges(rawRows, columns);
    const cells = rawRows.map((row) => row.map(cellValueFromMarkdown));
    for (const merge of merges) {
        const value = cells[merge.top][merge.left];
        for (let row = merge.top; row <= merge.bottom; row += 1) {
            for (let col = merge.left; col <= merge.right; col += 1) cells[row][col] = value;
        }
    }

    return normalizeTableModel({
        cells,
        merges,
        aligns: Array.from({ length: columns }, (_, col) => alignFromCell(separator[col] ?? '---')),
        rowHeights: Array.from(
            { length: rawRows.length },
            (_, row) => Math.max(
                MIN_ROW_HEIGHT,
                Number(parsedMetadata.value.rows?.[row]) || DEFAULT_ROW_HEIGHT,
            ),
        ),
        colWidths: Array.from(
            { length: columns },
            (_, col) => Math.max(
                MIN_COL_WIDTH,
                Number(parsedMetadata.value.cols?.[col]) || DEFAULT_COL_WIDTH,
            ),
        ),
        colSizing: Array.isArray(parsedMetadata.value.cols) ? 'explicit' : 'auto',
    });
}

function serializeAlign(align: TableAlign): string {
    if (align === 'left') return ':---';
    if (align === 'center') return ':---:';
    if (align === 'right') return '---:';
    return '---';
}

export function serializeMarkdownTable(model: TableModel): string {
    const content = model.cells.map((row) => row.map(cellValueToMarkdown));
    for (const merge of model.merges) {
        for (let row = merge.top; row <= merge.bottom; row += 1) {
            for (let col = merge.left; col <= merge.right; col += 1) {
                if (row === merge.top && col === merge.left) continue;
                content[row][col] = row > merge.top ? ROWSPAN : '';
            }
        }
    }
    const lines: string[] = [];
    for (let row = 0; row < content.length; row += 1) {
        lines.push(`| ${content[row].join(' | ')} |`);
        if (row === 0) lines.push(`| ${model.aligns.map(serializeAlign).join(' | ')} |`);
    }
    const hasCustomRows = model.rowHeights.some((height) => height !== DEFAULT_ROW_HEIGHT);
    const hasExplicitCols = model.colSizing === 'explicit';
    if (model.merges.length || hasCustomRows || hasExplicitCols) {
        const metadata: TableMetadata = {
            merges: model.merges.map((merge) => [merge.top, merge.left, merge.bottom, merge.right]),
        };
        if (hasCustomRows) metadata.rows = model.rowHeights.map((height) => Math.round(height));
        if (hasExplicitCols) metadata.cols = model.colWidths.map((width) => Math.round(width));
        lines.push(`<!--q-table:${JSON.stringify(metadata)}-->`);
    }
    return lines.join('\n');
}
