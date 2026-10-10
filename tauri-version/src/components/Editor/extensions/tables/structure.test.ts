import { describe, expect, it } from 'vitest';
import { columnCount, createEmptyTableModel, rowCount } from './model';
import {
    addTableColumn,
    addTableRow,
    removeTableColumn,
    removeTableRow,
} from './structure';

describe('table structure ops', () => {
    it('adds a body row and resumes on it', () => {
        const base = createEmptyTableModel(2, 1);
        const { model, resume } = addTableRow(base);
        expect(rowCount(model)).toBe(3);
        expect(resume).toEqual({ row: 2, col: 0 });
    });

    it('adds a column and resumes on the header cell', () => {
        const base = createEmptyTableModel(2, 1);
        const { model, resume } = addTableColumn(base);
        expect(model.aligns).toHaveLength(3);
        expect(columnCount(model)).toBe(3);
        expect(resume).toEqual({ row: 0, col: 2 });
    });

    it('removes a body row', () => {
        const base = addTableRow(createEmptyTableModel(2, 1)).model;
        const result = removeTableRow(base, 1, 0);
        expect(result).not.toBeNull();
        expect(rowCount(result!.model)).toBe(2);
        expect(result!.resume.row).toBe(1);
    });

    it('refuses to remove when only two rows remain', () => {
        const base = createEmptyTableModel(2, 1);
        expect(removeTableRow(base, 0, 0)).toBeNull();
        expect(removeTableRow(base, 1, 0)).toBeNull();
    });

    it('allows removing row 0 when more than two rows', () => {
        const base = addTableRow(createEmptyTableModel(2, 1)).model;
        const result = removeTableRow(base, 0, 0);
        expect(result).not.toBeNull();
        expect(rowCount(result!.model)).toBe(2);
    });

    it('removes a column', () => {
        const base = addTableColumn(createEmptyTableModel(2, 1)).model;
        const result = removeTableColumn(base, 0, 1);
        expect(result).not.toBeNull();
        expect(result!.model.aligns).toHaveLength(2);
        expect(result!.resume).toEqual({ row: 0, col: 1 });
    });
});
