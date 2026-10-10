import { Text } from '@codemirror/state';
import { parser } from '@lezer/markdown';
import { describe, expect, it } from 'vitest';
import {
    analyzeOutlineLines,
    continuationIndent,
    matchOutlineItem,
    openOutlineItem,
    outlineMarkdownConfig,
    reduceOutlineIndent,
} from './constructs';

describe('outline constructs', () => {
    it('recognizes bullet and ordered markers only after a complete prefix', () => {
        expect(matchOutlineItem('\t\t- text')).toEqual({
            indent: '\t\t',
            marker: { kind: 'bullet', text: '-', number: null },
            contentFrom: 4,
        });
        expect(matchOutlineItem('12. text')).toEqual({
            indent: '',
            marker: { kind: 'ordered', text: '12.', number: 12 },
            contentFrom: 4,
        });
        expect(matchOutlineItem('1) text')).toEqual({
            indent: '',
            marker: { kind: 'ordered', text: '1)', number: 1 },
            contentFrom: 3,
        });
        expect(matchOutlineItem('-')).toBeNull();
        expect(matchOutlineItem('1.')).toBeNull();
        expect(matchOutlineItem('1)')).toBeNull();
        expect(matchOutlineItem('* text')).toBeNull();
    });

    it('treats a second dash as literal document text', () => {
        const item = matchOutlineItem('- - text');

        expect(item?.contentFrom).toBe(2);
        expect('- - text'.slice(item?.contentFrom)).toBe('- text');
    });

    it('requires an unbroken parent chain for nested items', () => {
        expect(analyzeOutlineLines([
            '- root',
            '\t- child',
            '\t\t- grandchild',
            '- next root',
            '\t\t- missing level two',
        ]).map(item => item.showBullet)).toEqual([true, true, true, true, false]);

        expect(analyzeOutlineLines(['\t- orphan'])[0]).toMatchObject({
            level: 1,
            hasParent: false,
            showBullet: false,
        });
    });

    it('keeps parent chain across blank lines, breaks on plain text', () => {
        expect(analyzeOutlineLines([
            '- root',
            '\t- child',
            '',
            '\t- after blank',
            'plain',
            '\t- after text',
        ]).map(item => ({ showBullet: item.showBullet, hasParent: item.hasParent }))).toEqual([
            { showBullet: true, hasParent: true },
            { showBullet: true, hasParent: true },
            { showBullet: true, hasParent: true },
            { showBullet: false, hasParent: false },
        ]);
    });

    it('keeps following siblings when an item marker is removed', () => {
        expect(analyzeOutlineLines([
            '- root',
            '\t- child',
            '\tremoved marker',
            '\t- next child',
            '\t\t- grandchild',
        ]).map(item => item.hasParent)).toEqual([true, true, true, true]);

        expect(analyzeOutlineLines([
            '- root',
            '\tremoved parent marker',
            '\t\t- former child',
        ])[1]).toMatchObject({ hasParent: false });
    });

    it('reduces one indent level or reports root', () => {
        expect(reduceOutlineIndent('\t\t')).toBe('\t');
        expect(reduceOutlineIndent('\t')).toBe('');
        expect(reduceOutlineIndent('    ')).toBe('');
        expect(reduceOutlineIndent('')).toBeNull();
    });
});

describe('outline Markdown grammar', () => {
    const outlineParser = parser.configure(outlineMarkdownConfig);

    it('does not parse a tab-indented outline item as a code block', () => {
        const tree = outlineParser.parse('\t- child').toString();

        expect(tree).toContain('BulletList(ListItem(ListMark,Paragraph))');
        expect(tree).not.toContain('CodeBlock');
    });

    it('preserves indented code for ordinary text', () => {
        expect(outlineParser.parse('\tcode').toString()).toContain('CodeBlock(CodeText)');
    });

    it('does not promote paragraph + empty bullet into Setext heading', () => {
        const tree = outlineParser.parse('hello\n- ').toString();

        expect(tree).not.toContain('SetextHeading');
        expect(tree).toContain('BulletList(ListItem(ListMark,Paragraph))');
    });

    it('starts ordered lists from empty 1. and 1) under a paragraph', () => {
        expect(outlineParser.parse('hello\n1. ').toString())
            .toContain('OrderedList(ListItem(ListMark,Paragraph))');
        expect(outlineParser.parse('hello\n1) ').toString())
            .toContain('OrderedList(ListItem(ListMark,Paragraph))');
    });

    it('keeps horizontal rules after a paragraph when Setext is disabled', () => {
        expect(outlineParser.parse('hello\n---').toString()).toContain('HorizontalRule');
    });
});

describe('outline continuation lines', () => {
    it('reserves the marker width after the item indent', () => {
        expect(continuationIndent(matchOutlineItem('- text')!)).toBe('  ');
        expect(continuationIndent(matchOutlineItem('\t- text')!)).toBe('\t  ');
        expect(continuationIndent(matchOutlineItem('12. text')!)).toBe('    ');
    });

    it('continues an item only through that exact indent', () => {
        expect(analyzeOutlineLines([
            '- root',
            '  wrapped',
            '\t- child',
            '\t  wrapped child',
            'plain',
        ]).map(entry => [entry.lineNumber, entry.isContinuation])).toEqual([
            [1, false],
            [2, true],
            [3, false],
            [4, true],
        ]);
    });

    it('counts a line holding nothing but the indent, as right after a soft break', () => {
        expect(analyzeOutlineLines(['- root', '  '])[1]).toMatchObject({ isContinuation: true });
    });

    it('never continues an orphaned item or across an empty line', () => {
        expect(analyzeOutlineLines(['\t- orphan', '\t  wrapped'])).toHaveLength(1);
        expect(analyzeOutlineLines(['- root', '', '  wrapped'])).toHaveLength(1);
    });

    it('finds the item a continuation line belongs to', () => {
        const doc = Text.of(['- root', '  wrapped', '  wrapped more']);

        expect(openOutlineItem(doc, 2)?.marker.text).toBe('-');
        expect(openOutlineItem(doc, 3)?.marker.text).toBe('-');
    });

    it('reports no open item outside a list', () => {
        expect(openOutlineItem(Text.of(['plain', 'more']), 2)).toBeNull();
        expect(openOutlineItem(Text.of(['- root', '', '  stray']), 3)).toBeNull();
        expect(openOutlineItem(Text.of(['- root', '\tstray']), 2)).toBeNull();
    });
});
