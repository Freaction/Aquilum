// @vitest-environment happy-dom
import { EditorState } from '@codemirror/state';
import { EditorView } from '@codemirror/view';
import { afterEach, expect, it, vi } from 'vitest';
import { cursorTrailExtension } from './cursorTrail';

let view: EditorView;
afterEach(() => { view?.destroy(); vi.restoreAllMocks(); document.body.replaceChildren(); });
function create(reduced: boolean) {
  const matchMedia = window.matchMedia.bind(window);
  vi.spyOn(window, 'matchMedia').mockImplementation(query => {
    const media = matchMedia(query);
    if (query === '(prefers-reduced-motion: reduce)') Object.defineProperty(media, 'matches', { value: reduced });
    return media;
  });
  view = new EditorView({ parent: document.body, state: EditorState.create({ doc: 'abcdef', extensions: cursorTrailExtension }) });
  return vi.spyOn(view, 'requestMeasure');
}
it('does not create a layer or measure with reduced motion', () => {
  const measure = create(true);
  view.dispatch({ selection: { anchor: 5 } });
  expect(document.querySelector('.q-cursor-trail')).toBeNull();
  expect(measure.mock.calls.filter(([request]) => request)).toHaveLength(0);
});
it('does not create a trail for a one-character move', () => {
  const measure = create(false);
  view.dispatch({ selection: { anchor: 1 } });
  expect(document.querySelector('.q-cursor-trail')).toBeNull();
  expect(measure.mock.calls.filter(([request]) => request)).toHaveLength(0);
});
it('measures a jump asynchronously and cleans up its trail', () => {
  create(false);
  const coords = vi.spyOn(view, 'coordsAtPos').mockImplementation(pos => ({ left: pos * 10, right: pos * 10 + 1, top: 0, bottom: 20 }));
  let pending: Parameters<EditorView['requestMeasure']>[0];
  vi.spyOn(view, 'requestMeasure').mockImplementation(request => { if (request) pending = request; });
  view.dispatch({ selection: { anchor: 5 } });
  expect(coords).not.toHaveBeenCalled();
  expect(pending).toBeDefined();
  const value = pending!.read(view);
  pending!.write!(value, view);
  expect(document.querySelector('.q-cursor-trail')).not.toBeNull();
  view.destroy();
  expect(document.querySelector('.q-cursor-trail')).toBeNull();
});
