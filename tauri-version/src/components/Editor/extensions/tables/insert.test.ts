import { describe, expect, it } from 'vitest';
import { serializeTable } from './constructs';
import { parseMarkdownTable } from './markdown';
import { columnCount, createEmptyTableModel, rowCount } from './model';

describe('createEmptyTableModel', () => {
    it('builds a parseable 2×2 GFM table for the insert command', () => {
        const md = serializeTable(createEmptyTableModel(2, 1));
        const model = parseMarkdownTable(md);
        expect(model).not.toBeNull();
        expect(rowCount(model!)).toBe(2);
        expect(columnCount(model!)).toBe(2);
        expect(md).toContain('| --- | --- |');
        expect(model!.merges).toEqual([]);
    });

    it('supports custom size', () => {
        const model = createEmptyTableModel(3, 2);
        expect(rowCount(model)).toBe(3);
        expect(columnCount(model)).toBe(3);
    });
});
