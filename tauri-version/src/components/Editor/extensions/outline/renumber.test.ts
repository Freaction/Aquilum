import { EditorState, Text } from '@codemirror/state';
import { describe, expect, it } from 'vitest';
import { collectOrderedRenumberChanges } from './renumber';

describe('ordered outline numbering', () => {
    it('renumbers each level independently', () => {
        const doc = Text.of([
            '4. first',
            '\t8. nested',
            '\t9. nested',
            '7. second',
        ]);
        const state = EditorState.create({ doc });

        expect(state.update({
            changes: collectOrderedRenumberChanges(doc),
        }).state.doc.toString()).toBe([
            '1. first',
            '\t1. nested',
            '\t2. nested',
            '2. second',
        ].join('\n'));
    });

    it('keeps parenthesis suffix when renumbering', () => {
        const doc = Text.of(['4) first', '\t8) nested', '7) second']);
        const state = EditorState.create({ doc });
        expect(state.update({
            changes: collectOrderedRenumberChanges(doc),
        }).state.doc.toString()).toBe([
            '1) first',
            '\t1) nested',
            '2) second',
        ].join('\n'));
    });

    it('continues numbering across blank lines but resets after plain text', () => {
        const acrossBlank = Text.of(['1. a', '', '4. b']);
        expect(EditorState.create({ doc: acrossBlank }).update({
            changes: collectOrderedRenumberChanges(acrossBlank),
        }).state.doc.toString()).toBe('1. a\n\n2. b');

        const afterPlain = Text.of(['1. a', 'note', '4. b']);
        expect(EditorState.create({ doc: afterPlain }).update({
            changes: collectOrderedRenumberChanges(afterPlain),
        }).state.doc.toString()).toBe('1. a\nnote\n1. b');
    });
});
