import { EditorState } from '@codemirror/state';
import { describe, expect, it } from 'vitest';
import { bookCalloutInsertion } from './apply';

function insert(doc: string, anchor: number, head = anchor): EditorState {
    const state = EditorState.create({ doc, selection: { anchor, head } });
    return state.update(bookCalloutInsertion(state)).state;
}

describe('insertBookCallout', () => {
    it('inserts a book callout and selects its title placeholder', () => {
        const state = insert('', 0);

        expect(state.doc.toString()).toBe('> [!book] [[Название книги]]\n');
        expect(state.sliceDoc(
            state.selection.main.from,
            state.selection.main.to,
        )).toBe('Название книги');
    });

    it('starts a separate block when inserted inside a line', () => {
        const state = insert('Текст', 5);

        expect(state.doc.toString()).toBe(
            'Текст\n> [!book] [[Название книги]]\n',
        );
    });

    it('uses a single-line selection as the book title', () => {
        const title = 'Чистый код';
        const state = insert(title, 0, title.length);

        expect(state.doc.toString()).toBe('> [!book] [[Чистый код]]\n');
    });
});
