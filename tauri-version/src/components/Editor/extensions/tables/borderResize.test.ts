import { describe, expect, it } from 'vitest';
import { applyBorderResize, type BorderResizeState } from './borderResize';
import { serializeTable } from './constructs';
import { parseMarkdownTable } from './markdown';
import { MIN_COL_WIDTH, MIN_ROW_HEIGHT } from './model';

function autoTable() {
    return parseMarkdownTable([
        '| Действие | Результат |',
        '| --- | --- |',
        '| клик | каретка |',
    ].join('\n'))!;
}

function explicitTable() {
    return parseMarkdownTable([
        '| Действие | Результат |',
        '| --- | --- |',
        '| клик | каретка |',
        '<!--q-table:{"merges":[],"rows":[88,40,40],"cols":[115,158]}-->',
    ].join('\n'))!;
}

function drag(axis: 'row' | 'col', index: number, sizes: number[], delta: number) {
    const state: BorderResizeState = { axis, index, startPos: 500, startSizes: sizes };
    const event = { clientX: 500 + delta, clientY: 500 + delta } as MouseEvent;
    return { state, event };
}

describe('column sizing mode', () => {
    it('treats a hand-written table as auto and keeps metadata out of the file', () => {
        const model = autoTable();
        expect(model.colSizing).toBe('auto');
        expect(serializeTable(model)).not.toContain('q-table:');
    });

    it('treats stored cols as explicit and round-trips them', () => {
        const model = explicitTable();
        expect(model.colSizing).toBe('explicit');
        expect(model.colWidths).toEqual([115, 158]);
        expect(serializeTable(model)).toContain('"cols":[115,158]');
    });

    it('keeps row heights independent of column sizing', () => {
        const model = autoTable();
        const { state, event } = drag('row', 0, [88, 40], 12);
        const next = applyBorderResize(model, state, event)!;
        expect(next.rowHeights[0]).toBe(100);
        expect(next.colSizing).toBe('auto');
        expect(serializeTable(next)).toContain('"rows":[100,40]');
        expect(serializeTable(next)).not.toContain('"cols"');
    });
});

describe('border resize baseline', () => {
    it('starts from the rendered track size, not from the model default', () => {
        const model = autoTable();
        const { state, event } = drag('col', 0, [280, 262], 40);
        const next = applyBorderResize(model, state, event)!;
        expect(next.colSizing).toBe('explicit');
        expect(next.colWidths).toEqual([320, 262]);
    });

    it('materializes every rendered width so the dragged edge follows the cursor', () => {
        const model = autoTable();
        const { state, event } = drag('col', 1, [280, 262], -60);
        const next = applyBorderResize(model, state, event)!;
        expect(next.colWidths).toEqual([280, 202]);
    });

    it('clamps to the minimum track size', () => {
        const model = explicitTable();
        const cols = drag('col', 0, [115, 158], -400);
        expect(applyBorderResize(model, cols.state, cols.event)!.colWidths[0]).toBe(MIN_COL_WIDTH);
        const rows = drag('row', 1, [88, 40], -400);
        expect(applyBorderResize(model, rows.state, rows.event)!.rowHeights[1]).toBe(MIN_ROW_HEIGHT);
    });

    it('ignores a drag on a track that no longer exists', () => {
        const model = autoTable();
        const { state, event } = drag('col', 5, [280, 262], 20);
        expect(applyBorderResize(model, state, event)).toBeNull();
    });
});
