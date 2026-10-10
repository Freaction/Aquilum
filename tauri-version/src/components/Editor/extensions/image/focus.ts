import { Prec, StateEffect, StateField, type EditorState } from '@codemirror/state';
import { EditorView, keymap, type Command } from '@codemirror/view';
import { parseImageEmbed } from '../../../../modules/docs/imageEmbeds';

let pickedRoot: HTMLElement | null = null;

function pickedImage(): HTMLElement | null {
  if (pickedRoot && !pickedRoot.isConnected) pickedRoot = null;
  return pickedRoot;
}

function clearPickedImage(): void {
  if (pickedRoot) delete pickedRoot.dataset.selected;
  pickedRoot = null;
}

export function pickImage(root: HTMLElement): void {
  if (pickedRoot === root) return;
  clearPickedImage();
  pickedRoot = root;
  root.dataset.selected = 'true';
}

export const setImageSource = StateEffect.define<number | null>();

export const imageSourceField = StateField.define<number | null>({
  create: () => null,
  update(value, tr) {
    for (const effect of tr.effects) {
      if (effect.is(setImageSource)) return effect.value;
    }

    const caretLine = tr.newDoc.lineAt(tr.newSelection.main.head);
    if (tr.docChanged && parseImageEmbed(caretLine.text)) return caretLine.from;

    if (value === null) return null;
    const pos = tr.changes.mapPos(value, -1);
    return caretLine.from === pos ? pos : null;
  },
});

export function imageSourcePos(state: EditorState): number | null {
  return state.field(imageSourceField, false) ?? null;
}

const SOURCE_CARET_OFFSET = 2;

export function showImageSource(view: EditorView, pos: number): void {
  clearPickedImage();
  view.dispatch({
    selection: { anchor: pos + SOURCE_CARET_OFFSET },
    effects: setImageSource.of(pos),
    scrollIntoView: true,
  });
  view.focus();
}

const showPickedImageSource: Command = (view) => {
  const root = pickedImage();
  if (!root) return false;
  showImageSource(view, view.state.doc.lineAt(view.posAtDOM(root)).from);
  return true;
};

const deletePickedImage: Command = (view) => {
  const root = pickedImage();
  if (!root) return false;
  const line = view.state.doc.lineAt(view.posAtDOM(root));
  clearPickedImage();
  view.dispatch({
    changes: { from: line.from, to: Math.min(line.to + 1, view.state.doc.length) },
    selection: { anchor: line.from },
    userEvent: 'delete',
  });
  view.focus();
  return true;
};

const dropImageFocus: Command = (view) => {
  if (!pickedImage() && imageSourcePos(view.state) == null) return false;
  clearPickedImage();
  if (imageSourcePos(view.state) != null) view.dispatch({ effects: setImageSource.of(null) });
  return true;
};

export const imageFocusExtension = [
  imageSourceField,
  Prec.high(keymap.of([
    { key: 'Enter', run: showPickedImageSource },
    { key: 'Backspace', run: deletePickedImage },
    { key: 'Delete', run: deletePickedImage },
    { key: 'Escape', run: dropImageFocus },
  ])),
  EditorView.domEventHandlers({
    mousedown(event) {
      const target = event.target as HTMLElement | null;
      if (target?.closest('.q-md-image') !== pickedImage()) clearPickedImage();
      return false;
    },
  }),
];
