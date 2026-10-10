import { EditorSelection, EditorState } from '@codemirror/state';
import { describe, expect, it } from 'vitest';
import { imageSourceField, imageSourcePos, setImageSource } from './focus';

const DOC = '![700](a.jpg)\n\nтекст';

function stateOf(doc: string, pos: number) {
  return EditorState.create({
    doc,
    selection: EditorSelection.cursor(pos),
    extensions: [imageSourceField],
  });
}

describe('imageSourceField', () => {
  it('keeps the image when the selection lands inside its line', () => {
    const moved = stateOf(DOC, DOC.length)
      .update({ selection: EditorSelection.cursor(5) }).state;

    expect(imageSourcePos(moved)).toBeNull();
  });

  it('keeps the image when a selection spans it', () => {
    const spanned = stateOf(DOC, 0)
      .update({ selection: EditorSelection.range(0, DOC.length) }).state;

    expect(imageSourcePos(spanned)).toBeNull();
  });

  it('opens the source only when asked explicitly', () => {
    const shown = stateOf(DOC, DOC.length).update({
      selection: EditorSelection.cursor(2),
      effects: setImageSource.of(0),
    }).state;

    expect(imageSourcePos(shown)).toBe(0);
  });

  it('closes the source when the caret leaves the line', () => {
    const shown = stateOf(DOC, 2).update({ effects: setImageSource.of(0) }).state;
    const left = shown.update({ selection: EditorSelection.cursor(DOC.length) }).state;

    expect(imageSourcePos(left)).toBeNull();
  });

  it('keeps the source open while the line is typed out', () => {
    const typed = stateOf('![700](a.jpg', 12).update({
      changes: { from: 12, insert: ')' },
      selection: EditorSelection.cursor(13),
    }).state;

    expect(imageSourcePos(typed)).toBe(0);
  });

  it('leaves a pasted image closed, because the caret lands below it', () => {
    const pasted = stateOf('', 0).update({
      changes: { from: 0, insert: '![700](a.jpg)\n' },
      selection: EditorSelection.cursor(14),
    }).state;

    expect(imageSourcePos(pasted)).toBeNull();
  });
});
