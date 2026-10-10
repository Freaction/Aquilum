import { EditorState } from '@codemirror/state';
import { WidgetType } from '@codemirror/view';
import { describe, expect, it } from 'vitest';
import { editorFocusExtension, setEditorFocus } from './editorFocus';
import { collectPreviewReplaceDecorations } from './livePreviewEditableSpans';

class StubWidget extends WidgetType {
  toDOM(): HTMLElement {
    return document.createElement('div');
  }
}

const SPAN = { from: 0, to: 8 };

function stateWithCaretInside(focused: boolean): EditorState {
  const state = EditorState.create({
    doc: 'запрос\nпосле',
    selection: { anchor: 2 },
    extensions: [editorFocusExtension()],
  });
  return state.update({ effects: setEditorFocus.of(focused) }).state;
}

function widgetsIn(state: EditorState): number {
  return collectPreviewReplaceDecorations(state, [SPAN], () => new StubWidget()).length;
}

describe('блок под кареткой и фокус редактора', () => {
  it('показывает исходник, пока человек правит текст', () => {
    expect(widgetsIn(stateWithCaretInside(true))).toBe(0);
  });

  it('рисует себя, когда фокус ушёл из редактора', () => {
    expect(widgetsIn(stateWithCaretInside(false))).toBe(1);
  });

  it('свежее состояние считается расфокусированным', () => {
    const state = EditorState.create({
      doc: 'запрос\nпосле',
      selection: { anchor: 2 },
      extensions: [editorFocusExtension()],
    });
    expect(widgetsIn(state)).toBe(1);
  });
});
