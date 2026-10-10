// @vitest-environment happy-dom
import { EditorState } from '@codemirror/state';
import { EditorView } from '@codemirror/view';
import { afterEach, expect, it } from 'vitest';
import { hashtagExtension } from '../hashtags';
import { coloredTagsExtension } from './coloredTags';

let view: EditorView;
afterEach(() => { view?.destroy(); });
function tags(doc: string, colored = false) {
  view = new EditorView({ parent: document.body, state: EditorState.create({
    doc: '\n' + doc,
    extensions: [hashtagExtension, colored ? coloredTagsExtension({ enabled: true, mixNested: true, tagColors: { project: 'blue' } }) : []],
  }) });
  return [...view.dom.querySelectorAll<HTMLElement>('.q-cm-hashtag')];
}
it('recognizes nested tags as a whole', () => {
  expect(tags('#project/alpha #a/b/c').map(tag => tag.textContent)).toEqual(['#project/alpha', '#a/b/c']);
});
it('preserves ordinary tags, Cyrillic, digits, whitespace and boundaries', () => {
  expect(tags('#project #тег_123\t#42\nword#no #broken/ #a//b #x-y #tag.').map(tag => tag.textContent))
    .toEqual(['#project', '#тег_123', '#42']);
});
it('applies colors to the existing hashtag mark only when configured', () => {
  expect(tags('#project/alpha', true)[0].getAttribute('style'))
    .toMatch(/^--q-tag-color: color-mix\(in srgb, var\(--q-blue-500\) 60%, var\(--q-\w+-500\)\);?$/);
  view.destroy();
  expect(tags('#project')[0].getAttribute('style')).toBeNull();
});
