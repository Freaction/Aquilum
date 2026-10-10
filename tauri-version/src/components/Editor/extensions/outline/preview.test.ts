import { Text } from '@codemirror/state';
import { describe, expect, it } from 'vitest';
import { buildOutlineDecorations, getListIndentWidth } from './preview';

function decorationRanges(
    lines: string[],
    head = 0,
    visibleRanges?: readonly { from: number; to: number }[],
): Array<[number, number]> {
    const doc = Text.of(lines);
    const visible = visibleRanges ?? [{ from: 0, to: doc.length }];
    const ranges: Array<[number, number]> = [];
    buildOutlineDecorations(doc, visible, head).decorations.between(0, doc.length, (from, to) => {
        ranges.push([from, to]);
    });
    return ranges;
}

function atomicRanges(lines: string[]): Array<[number, number]> {
    const doc = Text.of(lines);
    const ranges: Array<[number, number]> = [];
    buildOutlineDecorations(doc, [{ from: 0, to: doc.length }])
        .continuationIndents.between(0, doc.length, (from, to) => {
            ranges.push([from, to]);
        });
    return ranges;
}

describe('outline preview', () => {
    it('заменяет чекбокс задачи виджетом и раскрывает его у каретки', () => {
        expect(decorationRanges(['- [ ] дело'], 8)).toEqual([
            [0, 0],
            [0, 2],
            [2, 5],
        ]);
        expect(decorationRanges(['- [ ] дело'], 3)).toEqual([
            [0, 0],
            [0, 2],
        ]);
    });

    it('пункт без чекбокса по-прежнему получает буллет', () => {
        expect(decorationRanges(['- дело'], 6)).toEqual([
            [0, 0],
            [0, 2],
        ]);
    });

    it('uses one fixed indentation width per level', () => {
        expect(getListIndentWidth('')).toBe('0px');
        expect(getListIndentWidth('\t')).toBe('var(--q-editor-list-indent-width)');
        expect(getListIndentWidth('\t\t')).toBe(
            'calc(var(--q-editor-list-indent-width) + var(--q-editor-list-indent-width))'
        );
    });

    it('builds sorted decorations for roots and nested items', () => {
        expect(decorationRanges(['- root', '\t- child'], 2)).toEqual([
            [0, 0],
            [0, 2],
            [7, 7],
            [7, 8],
            [8, 10],
        ]);
    });

    it('applies no outline layout to an orphaned nested item', () => {
        expect(decorationRanges(['\t- orphan'])).toEqual([[1, 2]]);
    });

    it('keeps nested formatting across a blank line', () => {
        expect(decorationRanges(['- root', '', '\t- child'], 2)).toEqual([
            [0, 0],
            [0, 2],
            [8, 8],
            [8, 9],
            [9, 11],
        ]);
    });

    it('drops nested formatting after plain text between items', () => {
        expect(decorationRanges(['- root', 'plain', '\t- orphan'], 2)).toEqual([
            [0, 0],
            [0, 2],
            [14, 15],
        ]);
    });

    it('shows the raw dash when the caret touches either side of the marker', () => {
        expect(decorationRanges(['- item'], 0)).toEqual([[0, 0], [0, 1]]);
        expect(decorationRanges(['- item'], 1)).toEqual([[0, 0], [0, 1]]);
        expect(decorationRanges(['- item'], 2)).toEqual([
            [0, 0],
            [0, 2],
        ]);
        expect(decorationRanges(['- item'], 6)).toEqual([
            [0, 0],
            [0, 2],
        ]);
    });

    it('shows raw 1. and 1) on either side of the marker, not after the space', () => {
        expect(decorationRanges(['1. item'], 0)).toEqual([[0, 0], [0, 2]]);
        expect(decorationRanges(['1. item'], 2)).toEqual([[0, 0], [0, 2]]);
        expect(decorationRanges(['1. item'], 3)).toEqual([
            [0, 0],
            [0, 3],
        ]);
        expect(decorationRanges(['1) item'], 0)).toEqual([[0, 0], [0, 2]]);
        expect(decorationRanges(['1) item'], 2)).toEqual([[0, 0], [0, 2]]);
        expect(decorationRanges(['1) item'], 3)).toEqual([
            [0, 0],
            [0, 3],
        ]);
        expect(decorationRanges(['1.'], 2)).toEqual([]);
        expect(decorationRanges(['1'], 1)).toEqual([]);
    });

    it('keeps adjacent bullet widgets when caret is on an ordered marker', () => {
        const lines = ['- **above**', '1) middle', '- **below**'];
        const doc = Text.of(lines);
        const middle = doc.line(2).from;
        expect(decorationRanges(lines, middle)).toEqual([
            [0, 0],
            [0, 2],
            [doc.line(2).from, doc.line(2).from],
            [doc.line(2).from, doc.line(2).from + 2],
            [doc.line(3).from, doc.line(3).from],
            [doc.line(3).from, doc.line(3).from + 2],
        ]);
    });

    it('removes all bullet formatting after the marker space is deleted', () => {
        expect(decorationRanges(['-'], 1)).toEqual([]);
    });

    it('keeps preview formatting on a sibling after the previous marker is removed', () => {
        const lines = ['- parent', '\tchild without marker', '\t- next child'];
        const next = Text.of(lines).line(3).from;

        expect(decorationRanges(lines, next + 3)).toEqual([
            [0, 0], [0, 2],
            [next, next], [next, next + 1], [next + 1, next + 3],
        ]);
        expect(decorationRanges(lines, next + 3, [{ from: next, to: Text.of(lines).length }])).toEqual([
            [next, next], [next, next + 1], [next + 1, next + 3],
        ]);
    });

    it('window analysis matches whole-document analysis deep in the doc', () => {
        const lines: string[] = [];
        for (let block = 0; block < 40; block++) {
            lines.push(`Paragraph ${block}`, '- one', '\t- two', '\t\t- three', '');
        }
        const doc = Text.of(lines);
        const windowStart = doc.line(5 * 30 + 3).from;
        const windowEnd = doc.line(5 * 30 + 6).to;

        const windowed = decorationRanges(lines, 0, [{ from: windowStart, to: windowEnd }]);
        const whole = decorationRanges(lines)
            .filter(([from, to]) => from >= windowStart && to <= windowEnd);

        expect(windowed).toEqual(whole);
        expect(windowed.length).toBeGreaterThan(0);
    });
});

describe('outline continuation preview', () => {
    it('lays out a continuation line like its item and collapses the indent', () => {
        const lines = ['- root', '  wrapped'];
        const second = Text.of(lines).line(2).from;

        expect(decorationRanges(lines, 2)).toEqual([
            [0, 0],
            [0, 2],
            [second, second],
            [second, second + 2],
        ]);
    });

    it('keeps a nested continuation on its own item', () => {
        const lines = ['- root', '\t- child', '\t  wrapped'];
        const doc = Text.of(lines);
        const child = doc.line(2).from;
        const wrapped = doc.line(3).from;

        expect(decorationRanges(lines, 2)).toEqual([
            [0, 0],
            [0, 2],
            [child, child],
            [child, child + 1],
            [child + 1, child + 3],
            [wrapped, wrapped],
            [wrapped, wrapped + 3],
        ]);
    });

    it('reports the continuation indent as one atomic range', () => {
        const second = Text.of(['- root', '  wrapped']).line(2).from;

        expect(atomicRanges(['- root', '  wrapped'])).toEqual([[second, second + 2]]);
        expect(atomicRanges(['- root', '- sibling'])).toEqual([]);
    });
});
