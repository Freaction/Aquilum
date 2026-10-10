import type { EditorView } from '@codemirror/view';

export function resolveEditorScrollElement(container: HTMLElement): HTMLElement {
  const book = container.querySelector('.q-book-page:not(.q-book-page--inactive)');
  return book ? book as HTMLElement : container;
}

export function captureScroll(view: EditorView, scrollElement: HTMLElement) {
  const scrollportTop = scrollElement.getBoundingClientRect().top;
  const localTop = scrollportTop - view.scrollDOM.getBoundingClientRect().top;
  const block = view.lineBlockAtHeight(Math.max(0, localTop));
  const coords = view.coordsAtPos(block.from);
  return {
    anchor: block.from,
    offset: coords ? scrollportTop - coords.top : localTop - block.top,
  };
}
