import { markdown } from '@codemirror/lang-markdown';
import { EditorState } from '@codemirror/state';
import { Table } from '@lezer/markdown';
import { describe, expect, it } from 'vitest';
import { findTablesInDoc, sanitizeCellValue, serializeTable } from './constructs';
import { parseMarkdownTable } from './markdown';
import { setCell } from './model';
import { isSeparatorRow, splitTableRow } from './rows';

function stateWith(doc: string): EditorState {
    return EditorState.create({
        doc,
        extensions: [markdown({ extensions: [Table] })],
    });
}

it('keeps escaped wiki aliases and inline code in one cell through parse and serialize', () => {
    const doc = '| link | code |\n| --- | --- |\n| [[Note\\|alias]] | `a\\|b` |';
    const model = parseMarkdownTable(doc)!;
    expect(model.cells[1]).toEqual(['[[Note\\|alias]]', '`a\\|b`']);
    expect(serializeTable(model)).toContain(String.raw`[[Note\|alias]]`);
    expect(parseMarkdownTable(serializeTable(model))!.cells).toEqual(model.cells);
});

it('splits every unescaped pipe even inside wiki links and inline code', () => {
    expect(splitTableRow('| [[a|b]] | `a|b` |')).toEqual(['[[a', 'b]]', '`a', 'b`']);
    expect(splitTableRow(String.raw`| a\|b | c |`)).toEqual([String.raw`a\|b`, 'c']);
    expect(splitTableRow(String.raw`a\|`)).toEqual([String.raw`a\|`]);
});

it('escapes raw pipes in entered wiki aliases and inline code when saving a cell', () => {
    const model = parseMarkdownTable('| link | code |\n| --- | --- |\n| note | code |')!;
    const updated = setCell(setCell(model, 1, 0, '[[Note|alias]]'), 1, 1, '`a|b`');
    const saved = serializeTable(updated);
    expect(saved).toContain(String.raw`[[Note\|alias]]`);
    expect(saved).toContain(String.raw`a\|b`);
    expect(parseMarkdownTable(saved)!.cells[1]).toEqual([String.raw`[[Note\|alias]]`, '`a\\|b`']);
    expect(sanitizeCellValue('outside|text')).toBe('outside∣text');
});

describe('table constructs', () => {
    it('splits pipe rows and detects separators', () => {
        expect(splitTableRow('| A | B |')).toEqual(['A', 'B']);
        expect(splitTableRow('A | B')).toEqual(['A', 'B']);
        expect(isSeparatorRow('| --- | :---: |')).toBe(true);
        expect(isSeparatorRow('| A | B |')).toBe(false);
    });

    it('parses only complete GFM tables (H1)', () => {
        expect(parseMarkdownTable('| A | B |\n| --- | --- |')).toBeNull();
        expect(parseMarkdownTable('| A | B |\n| --- | --- |\n| 1 | 2 |')!.cells).toEqual([
            ['A', 'B'],
            ['1', '2'],
        ]);
        expect(parseMarkdownTable('| A | B |\n| :--- | ---: |\n| 1 | 2 |')?.aligns).toEqual([
            'left',
            'right',
        ]);
    });

    it('normalizes ragged column counts', () => {
        const model = parseMarkdownTable('| A | B | C |\n| --- | --- | --- |\n| 1 | 2 |\n| x | y | z | w |');
        expect(model!.cells).toEqual([
            ['A', 'B', 'C'],
            ['1', '2', ''],
            ['x', 'y', 'z'],
        ]);
    });

    it('roundtrips serialize → parse (H4 shape)', () => {
        const source = '| Name | Age |\n| --- | --- |\n| Ada | 36 |';
        const model = parseMarkdownTable(source);
        expect(model).not.toBeNull();
        const again = parseMarkdownTable(serializeTable(model!));
        expect(again!.cells).toEqual(model!.cells);
        expect(again?.aligns).toEqual(model?.aligns);
    });

    it('sanitizes pipes and newlines so GFM columns stay intact', () => {
        expect(sanitizeCellValue('a|b\nc')).toBe('a∣b c');
        const broken = parseMarkdownTable('| A | B |\n| --- | --- |\n| 1 | 2 |')!;
        const updated = setCell(broken, 1, 0, 'x|y\nz');
        const again = parseMarkdownTable(serializeTable(updated));
        expect(again!.cells[1][0]).toBe('x∣y z');
        expect(again!.cells[1]).toHaveLength(2);
    });

    it('finds adjacent tables without merging (H4)', () => {
        const state = stateWith([
            '| A | B |',
            '| --- | --- |',
            '| 1 | 2 |',
            '| C | D |',
            '| --- | --- |',
            '| 3 | 4 |',
        ].join('\n'));
        const ranges = findTablesInDoc(state.doc);
        expect(ranges).toHaveLength(2);
        expect(ranges[0].model.cells[0]).toEqual(['A', 'B']);
        expect(ranges[1].model.cells[0]).toEqual(['C', 'D']);
        expect(ranges[0].to).toBe(state.doc.line(4).from);
        expect(ranges[0].contentTo).toBe(state.doc.line(3).to);
        expect(ranges[1].to).toBe(state.doc.length);
    });

});
