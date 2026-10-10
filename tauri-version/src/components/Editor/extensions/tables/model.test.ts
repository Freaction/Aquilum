import { describe, expect, it } from 'vitest';
import {
    DEFAULT_COL_WIDTH,
    deleteColumn,
    destFromInsertGap,
    insertColumn,
    insertRow,
    moveColumns,
    moveRows,
    rowCount,
    setCell,
    withExplicitColWidths,
} from './model';
import { serializeTable } from './constructs';
import { parseMarkdownTable } from './markdown';
import { expandedRowBlock, isValidInsertGap } from './stripReorder';

function sample() {
    return parseMarkdownTable('| A | B |\n| --- | --- |\n| 1 | 2 |')!;
}

describe('table model', () => {
    it('updates cells and inserts structure', () => {
        const model = setCell(sample(), 1, 0, 'x');
        expect(model.cells[1][0]).toBe('x');
        expect(rowCount(insertRow(sample(), 2))).toBe(3);
        expect(insertColumn(sample(), 1).aligns).toHaveLength(3);
    });

    it('rejects deleting the last data row or last column', () => {
        expect(deleteColumn(sample(), 0)?.aligns).toHaveLength(1);
        const oneCol = deleteColumn(sample(), 0)!;
        expect(deleteColumn(oneCol, 0)).toBeNull();
    });

    it('moves a row block and remaps merges', () => {
        const model = parseMarkdownTable([
            '| a | b | c |',
            '| --- | --- | --- |',
            '| 1 | 2 | 3 |',
            '| 4 | 5 | 6 |',
            '| 7 | 8 | 9 |',
            '<!--q-table:{"merges":[[1,0,1,1]]}-->',
        ].join('\n'))!;

        const moved = moveRows(model, 1, 1, 2)!;
        expect(moved.cells[2][0]).toBe('1');
        expect(moved.cells[1][0]).toBe('4');
        expect(moved.merges).toEqual([{ top: 2, left: 0, bottom: 2, right: 1 }]);
    });

    it('moves a column block with aligns', () => {
        const model = parseMarkdownTable([
            '| a | b | c |',
            '| :--- | ---: | --- |',
            '| 1 | 2 | 3 |',
        ].join('\n'))!;
        const moved = moveColumns(model, 0, 0, 2)!;
        expect(moved.cells[1]).toEqual(['2', '3', '1']);
        expect(moved.aligns[2]).toBe('left');
    });

    it('maps insert gaps onto final destinations', () => {
        expect(destFromInsertGap(1, 2, 0)).toBe(0);
        expect(destFromInsertGap(1, 2, 1)).toBe(1);
        expect(destFromInsertGap(1, 2, 3)).toBe(1);
        expect(destFromInsertGap(1, 2, 4)).toBe(2);
    });

    it('sets column widths and remaps them on insert/delete/move', () => {
        const wide = withExplicitColWidths(sample(), [120]);
        expect(wide.colWidths[0]).toBe(120);
        expect(wide.colWidths[1]).toBe(DEFAULT_COL_WIDTH);

        const inserted = insertColumn(wide, 1);
        expect(inserted.colWidths).toEqual([120, DEFAULT_COL_WIDTH, DEFAULT_COL_WIDTH]);

        const deleted = deleteColumn(inserted, 1)!;
        expect(deleted.colWidths).toEqual([120, DEFAULT_COL_WIDTH]);

        const moved = moveColumns(deleted, 0, 0, 1)!;
        expect(moved.colWidths).toEqual([DEFAULT_COL_WIDTH, 120]);
    });

    it('round-trips custom col widths through markdown metadata', () => {
        const model = withExplicitColWidths(sample(), [DEFAULT_COL_WIDTH, 96]);
        const text = serializeTable(model);
        expect(text).toContain('"cols":');
        const parsed = parseMarkdownTable(text)!;
        expect(parsed.colWidths[1]).toBe(96);
        expect(parsed.colWidths[0]).toBe(DEFAULT_COL_WIDTH);
    });
});

describe('strip reorder expand', () => {
    it('couples rows that share a vertical merge', () => {
        const model = parseMarkdownTable([
            '| a | b |',
            '| --- | --- |',
            '| 1 | x |',
            '| ^^ | y |',
            '| 3 | z |',
            '<!--q-table:{"merges":[[1,0,2,0]]}-->',
        ].join('\n'))!;
        expect(expandedRowBlock(model, 1)).toEqual({ top: 1, bottom: 2 });
        expect(expandedRowBlock(model, 2)).toEqual({ top: 1, bottom: 2 });
        expect(expandedRowBlock(model, 3)).toEqual({ top: 3, bottom: 3 });
    });

    it('blocks insert gaps that split an external merge', () => {
        const model = parseMarkdownTable([
            '| a | b | c |',
            '| --- | --- | --- |',
            '| 1 |  | 3 |',
            '<!--q-table:{"merges":[[1,0,1,1]]}-->',
        ].join('\n'))!;
        expect(isValidInsertGap(model, 'col', 2, 2, 1)).toBe(false);
        expect(isValidInsertGap(model, 'col', 2, 2, 0)).toBe(true);
        expect(isValidInsertGap(model, 'col', 2, 2, 2)).toBe(false);
        expect(isValidInsertGap(model, 'col', 2, 2, 3)).toBe(false);
    });
});
