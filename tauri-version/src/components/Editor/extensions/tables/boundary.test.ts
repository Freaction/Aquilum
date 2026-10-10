import { EditorSelection, EditorState } from '@codemirror/state';
import { describe, expect, it } from 'vitest';
import {
    blankLineInsertPos,
    collectBlankLineChanges,
    docStartsWithTable,
    tableBoundaryExtension,
    tableCaretGuard,
    tableToSelectOnBackspace,
} from './boundary';
import { findTablesInDoc, tableOwnsDocEnd } from './constructs';

const TABLE = '| A | B |\n| --- | --- |\n| 1 | 2 |';

describe('line above a leading table', () => {
    it('reports the table start when the caret would land inside it', () => {
        const state = EditorState.create({ doc: TABLE + '\n' });
        expect(docStartsWithTable(state)).toBe(true);
    });

    it('reports nothing when the body does not start with a table', () => {
        const state = EditorState.create({ doc: 'Абзац\n\n' + TABLE + '\n' });
        expect(docStartsWithTable(state)).toBe(false);
    });

    it('opening the line puts the caret before the table, so the guard leaves it alone', () => {
        const state = EditorState.create({
            doc: TABLE + '\n',
            extensions: [tableCaretGuard],
        });
        expect(docStartsWithTable(state)).toBe(true);

        const next = state.update({
            changes: { from: 0, insert: '\n' },
            selection: EditorSelection.cursor(0),
        }).state;

        expect(next.selection.main.head).toBe(0);
        expect(next.doc.line(1).text).toBe('');
        expect(next.doc.line(2).text).toBe('| A | B |');
        expect(findTablesInDoc(next.doc)[0].from).toBe(1);
    });
});

describe('table blank line boundary', () => {
    it('detects when a table owns the last line', () => {
        const noBreak = EditorState.create({ doc: TABLE });
        const tables = findTablesInDoc(noBreak.doc);
        expect(tables).toHaveLength(1);
        expect(tableOwnsDocEnd(noBreak.doc, tables[0])).toBe(true);
        expect(collectBlankLineChanges(noBreak.doc)).toEqual([
            { from: noBreak.doc.length, insert: '\n' },
        ]);

        const withBreak = EditorState.create({ doc: `${TABLE}\n` });
        const after = findTablesInDoc(withBreak.doc);
        expect(tableOwnsDocEnd(withBreak.doc, after[0])).toBe(false);
        expect(collectBlankLineChanges(withBreak.doc)).toEqual([]);
        expect(blankLineInsertPos(withBreak.doc, after[0]!)).toBeNull();
    });

    it('inserts a blank line between a table and following text', () => {
        const state = EditorState.create({
            doc: 'x',
            extensions: [tableBoundaryExtension],
        }).update({
            changes: { from: 0, to: 1, insert: `${TABLE}\nhello` },
        }).state;

        expect(state.doc.toString()).toBe(`${TABLE}\n\nhello`);
        expect(collectBlankLineChanges(state.doc)).toEqual([]);
    });

    it('appends a trailing break when table would own EOF', () => {
        const state = EditorState.create({
            doc: 'hi\n',
            extensions: [tableBoundaryExtension],
        });
        const next = state.update({
            changes: { from: 0, to: state.doc.length, insert: TABLE },
        }).state;
        expect(next.doc.toString().endsWith('\n')).toBe(true);
        expect(collectBlankLineChanges(next.doc)).toEqual([]);
    });
});

describe('table caret guard', () => {
    it('snaps collapsed caret from hidden table source to the line below', () => {
        const state = EditorState.create({
            doc: `${TABLE}\n`,
            extensions: [tableCaretGuard],
        });
        const table = findTablesInDoc(state.doc)[0]!;
        const next = state.update({
            selection: EditorSelection.cursor(table.from),
        }).state;
        expect(next.selection.main.head).toBe(table.to);
    });

    it('allows a non-empty selection covering the table', () => {
        const state = EditorState.create({
            doc: `${TABLE}\n`,
            extensions: [tableCaretGuard],
        });
        const table = findTablesInDoc(state.doc)[0]!;
        const next = state.update({
            selection: EditorSelection.range(table.from, table.to),
        }).state;
        expect(next.selection.main.from).toBe(table.from);
        expect(next.selection.main.to).toBe(table.to);
    });
});

describe('table backspace select', () => {
    it('targets the table when caret is on the line below', () => {
        const state = EditorState.create({ doc: `${TABLE}\n` });
        const table = findTablesInDoc(state.doc)[0]!;
        const atBoundary = state.update({
            selection: EditorSelection.cursor(table.to),
        }).state;
        expect(tableToSelectOnBackspace(atBoundary)).toEqual(table);
    });

    it('does not target when selection is already non-empty', () => {
        const base = EditorState.create({ doc: `${TABLE}\n` });
        const table = findTablesInDoc(base.doc)[0]!;
        const selected = base.update({
            selection: EditorSelection.range(table.from, table.to),
        }).state;
        expect(tableToSelectOnBackspace(selected)).toBeNull();
    });
});
