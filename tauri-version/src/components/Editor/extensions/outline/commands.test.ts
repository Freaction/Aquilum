import { indentLess, indentMore } from '@codemirror/commands';
import { indentUnit } from '@codemirror/language';
import { EditorSelection, EditorState, type StateCommand } from '@codemirror/state';
import { describe, expect, it } from 'vitest';
import {
    breakOutlineLine,
    continueOutlineItem,
    outlineKeyBindings,
} from './commands';

function run(command: StateCommand, state: EditorState): EditorState {
    let nextState = state;
    const handled = command({
        state,
        dispatch: transaction => {
            nextState = transaction.state;
        },
    });
    expect(handled).toBe(true);
    return nextState;
}

function cursorState(doc: string): EditorState {
    return EditorState.create({
        doc,
        selection: EditorSelection.cursor(doc.length),
        extensions: [indentUnit.of('\t')],
    });
}

function deletePreviousCharacter(state: EditorState): EditorState {
    const position = state.selection.main.head;
    return state.update({
        changes: { from: position - 1, to: position, insert: '' },
        selection: { anchor: position - 1 },
    }).state;
}

describe('outline editing', () => {
    it('continues bullet and ordered items on the same level', () => {
        expect(run(continueOutlineItem, cursorState('\t\t- first')).doc.toString()).toBe(
            '\t\t- first\n\t\t- '
        );
        expect(run(continueOutlineItem, cursorState('\t2. second')).doc.toString()).toBe(
            '\t2. second\n\t3. '
        );
        expect(run(continueOutlineItem, cursorState('1) first')).doc.toString()).toBe(
            '1) first\n2) '
        );
    });

    it('stores typed dashes literally without an input correction layer', () => {
        let state = cursorState('');
        for (const text of ['-', ' ', '- ']) {
            const position = state.selection.main.head;
            state = state.update({
                changes: { from: position, insert: text },
                selection: { anchor: position + text.length },
            }).state;
        }

        expect(state.doc.toString()).toBe('- - ');
    });

    it('adds and removes exactly one real tab', () => {
        const secondLevel = run(indentMore, cursorState('- item'));

        expect(secondLevel.doc.toString()).toBe('\t- item');
        expect(run(indentLess, secondLevel).doc.toString()).toBe('- item');
    });

    it('outdents an empty nested item on Enter and exits at root', () => {
        expect(run(continueOutlineItem, cursorState('\t\t- ')).doc.toString()).toBe('\t- ');
        expect(run(continueOutlineItem, cursorState('\t2. ')).doc.toString()).toBe('2. ');
        expect(run(continueOutlineItem, cursorState('\t1) ')).doc.toString()).toBe('1) ');
        expect(run(continueOutlineItem, cursorState('- ')).doc.toString()).toBe('');
    });

    it('outdent on empty nested item keeps following siblings intact', () => {
        const doc = '\t- \n\t- sibling\n- root';
        const state = EditorState.create({
            doc,
            selection: EditorSelection.cursor(3),
            extensions: [indentUnit.of('\t')],
        });
        const next = run(continueOutlineItem, state);
        expect(next.doc.toString()).toBe('- \n\t- sibling\n- root');
    });

    it('does not override Backspace', () => {
        expect(outlineKeyBindings.some(binding => binding.key === 'Backspace')).toBe(false);
    });

    it('deletes bullet and ordered prefixes one character at a time', () => {
        const bulletWithoutSpace = deletePreviousCharacter(cursorState('- '));
        expect(bulletWithoutSpace.doc.toString()).toBe('-');

        const numberWithoutSpace = deletePreviousCharacter(cursorState('1. '));
        expect(numberWithoutSpace.doc.toString()).toBe('1.');
        expect(deletePreviousCharacter(numberWithoutSpace).doc.toString()).toBe('1');

        const parenWithoutSpace = deletePreviousCharacter(cursorState('1) '));
        expect(parenWithoutSpace.doc.toString()).toBe('1)');
        expect(deletePreviousCharacter(parenWithoutSpace).doc.toString()).toBe('1');
    });
});

describe('outline soft breaks', () => {
    it('opens the next line under the text of the item', () => {
        expect(run(breakOutlineLine, cursorState('- first')).doc.toString()).toBe('- first\n  ');
        expect(run(breakOutlineLine, cursorState('\t12. first')).doc.toString())
            .toBe('\t12. first\n\t    ');
    });

    it('repeats the indent on a line that already continues an item', () => {
        expect(run(breakOutlineLine, cursorState('- first\n  second')).doc.toString())
            .toBe('- first\n  second\n  ');
    });

    it('opens the next item when Enter follows a soft break', () => {
        expect(run(continueOutlineItem, cursorState('- first\n  second')).doc.toString())
            .toBe('- first\n  second\n- ');
        expect(run(continueOutlineItem, cursorState('\t2. first\n\t   second')).doc.toString())
            .toBe('\t2. first\n\t   second\n\t3. ');
    });

    it('leaves plain text and the marker itself to the default binding', () => {
        const dispatch = () => {};
        expect(breakOutlineLine({ state: cursorState('plain'), dispatch })).toBe(false);
        expect(continueOutlineItem({ state: cursorState('plain'), dispatch })).toBe(false);

        const insideMarker = EditorState.create({
            doc: '- first',
            selection: EditorSelection.cursor(1),
        });
        expect(breakOutlineLine({ state: insideMarker, dispatch })).toBe(false);
    });

    it('binds the soft break to Shift-Enter', () => {
        expect(outlineKeyBindings.some(binding => binding.key === 'Shift-Enter')).toBe(true);
    });
});
