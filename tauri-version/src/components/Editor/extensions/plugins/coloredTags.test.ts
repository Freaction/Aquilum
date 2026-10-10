// @vitest-environment happy-dom
import { describe, expect, it } from 'vitest';
import { tagColor, tagHash, coloredTagsExtension } from './coloredTags';
import { EditorView } from '@codemirror/view';

const settings = { enabled: true, mixNested: true, tagColors: {} };
it('gives colored tag rules greater specificity than the base hashtag', () => {
  const view = new EditorView({ parent: document.body, extensions: coloredTagsExtension(settings) });
  try {
    const css = [...document.styleSheets].flatMap(sheet => [...sheet.cssRules].map(rule => rule.cssText)).join('\n');
    expect(css).toContain('.q-cm-hashtag.q-cm-hashtag--colored');
  } finally { view.destroy(); }
});
describe('colored tags', () => {
  it('uses stable FNV-1a hashes', () => {
    expect(tagHash('')).toBe(2166136261);
    expect(tagHash('hello')).toBe(1335831723);
    expect(tagColor('project', settings)).toBe(tagColor('project', { ...settings }));
  });
  it('mixes nested colors and honors pinned colors', () => {
    const pinned = { ...settings, tagColors: { project: 'blue', 'project/alpha': 'red' } };
    expect(tagColor('project', pinned)).toBe('var(--q-blue-500)');
    expect(tagColor('project/alpha', { ...settings, tagColors: { project: 'blue' } }))
      .toMatch(/^color-mix\(in srgb, var\(--q-blue-500\) 60%, var\(--q-\w+-500\)\)$/);
    expect(tagColor('project/alpha', pinned)).toBe('var(--q-red-500)');
    expect(tagColor('project/alpha', { ...pinned, mixNested: false, tagColors: { project: 'blue' } }))
      .toBe('var(--q-blue-500)');
  });
  it('does not interpolate invalid tokens into CSS', () => {
    expect(tagColor('project', { ...settings, tagColors: { project: 'blue);color:bad' } }))
      .toBe(tagColor('project', settings));
  });
});
