// @vitest-environment happy-dom
import { syntaxTree, syntaxTreeAvailable } from '@codemirror/language';
import { Compartment, EditorState, StateEffect } from '@codemirror/state';
import { EditorView } from '@codemirror/view';
import { afterEach, expect, it, vi } from 'vitest';
import { editorMarkdownSupport } from '../../markdownConfig';
import { codeBlockExtension } from '../../codeBlock';
import { codeStylerExtension } from './index';
import { revealAtCaretFacet } from '../../livePreviewConfig';
import { t } from '../../../../../i18n';

vi.mock('@codemirror/language', async importOriginal => {
  const actual = await importOriginal<typeof import('@codemirror/language')>();
  return { ...actual, syntaxTree: vi.fn(actual.syntaxTree), syntaxTreeAvailable: vi.fn(actual.syntaxTreeAvailable) };
});

let view: EditorView;
afterEach(() => { view?.destroy(); vi.useRealTimers(); vi.restoreAllMocks(); vi.resetAllMocks(); });
const settings = { enabled: true, lineNumbers: false, copyButton: true, header: true };
function create(doc: string, overrides = {}) {
  view = new EditorView({ parent: document.body, state: EditorState.create({ doc, extensions: [editorMarkdownSupport, codeBlockExtension, codeStylerExtension({ ...settings, ...overrides })] }) });
  return view;
}

it('hides the live status and restores the copy icon after 1500 ms', async () => {
  vi.useFakeTimers();
  vi.spyOn(navigator.clipboard, 'writeText').mockResolvedValue();
  create('```js\nbody\n```');
  const button = view.dom.querySelector<HTMLButtonElement>('.q-md-code-copy')!;
  const original = button.innerHTML;
  button.click();
  await Promise.resolve();
  const status = view.dom.querySelector<HTMLElement>('.q-code-styler-status[aria-live]')!;
  expect(status.classList.contains('q-code-styler-status')).toBe(true);
  expect(status.textContent).toBe(t('plugins.codeStyler.copied'));
  expect(button.innerHTML).not.toBe(original);
  vi.advanceTimersByTime(1499);
  expect(button.innerHTML).not.toBe(original);
  vi.advanceTimersByTime(1);
  expect(button.innerHTML).toBe(original);
  expect(status.textContent).toBe('');
});

it('joins a caption to the block and floats copying when there is no caption', () => {
  create('```js\nbody\n```\n\n```\nplain\n```');
  expect(view.dom.querySelectorAll('.q-code-styler-header')).toHaveLength(1);
  expect(view.dom.querySelector('.q-md-code-line--first.q-code-styler-captioned')).not.toBeNull();
  const floating = view.dom.querySelector('.q-code-styler-floating')!;
  expect(floating.closest('.q-md-code-line--first')).not.toBeNull();
  expect(floating.querySelector('.q-md-code-copy')).not.toBeNull();
  const css = [...document.styleSheets].flatMap(sheet => [...sheet.cssRules].map(rule => rule.cssText)).join('\n');
  expect(css).toContain('.q-code-styler-status');
  expect(css).toContain('clip-path: inset(50%)');
  expect(css).toContain('border-top-left-radius: var(--q-block-radius)');
});
it('gives numbering and highlight rules greater specificity than the code block theme', () => {
  create('```js ln:true hl:1\nconst value = 1;\n```');
  const css = [...document.styleSheets].flatMap(sheet => [...sheet.cssRules].map(rule => rule.cssText)).join('\n');
  expect(css).toContain('.cm-line.q-md-code-line.q-md-code-numbered {');
  expect(css).toContain('.cm-line.q-md-code-line.q-md-code-numbered::after');
  expect(css).toContain('.cm-line.q-md-code-line.q-md-code-line--hl::before');
});
it('copies the body without either fence and decorates only body line numbers', async () => {
  const write = vi.spyOn(navigator.clipboard, 'writeText').mockResolvedValue();
  const doc = '```js title:"a b" ln:5 hl:2\none\ntwo\n```';
  create(doc);
  expect(view.dom.querySelector('.q-code-styler-header')!.textContent).toContain('a b');
  const numbered = [...view.dom.querySelectorAll<HTMLElement>('[data-ln]')];
  expect(numbered.map(line => line.dataset.ln)).toEqual(['5', '6']);
  expect(view.dom.querySelector('.q-md-code-line--hl')!.textContent).toBe('two');
  view.dom.querySelector<HTMLButtonElement>(`[aria-label="${t('plugins.codeStyler.copy')}"]`)!.click();
  await Promise.resolve();
  expect(write).toHaveBeenCalledWith('one\ntwo\n');
  expect(view.state.doc.toString()).toBe(doc);
});

it('reveals only the hovered block copy button and permits keyboard focus', () => {
  create('```\none\n```\n\n```\ntwo\n```');
  const roots = [...view.dom.querySelectorAll<HTMLElement>('.q-code-styler-floating')];
  const lines = [...view.dom.querySelectorAll<HTMLElement>('.q-md-code-line')];
  lines[1].dispatchEvent(new MouseEvent('mouseover', { bubbles: true }));
  expect(roots.map(root => root.classList.contains('is-hovered'))).toEqual([true, false]);
  lines.find(line => line.textContent === 'two')!.dispatchEvent(new MouseEvent('mouseover', { bubbles: true }));
  expect(roots.map(root => root.classList.contains('is-hovered'))).toEqual([false, true]);
  const button = roots[1].querySelector<HTMLButtonElement>('button')!;
  button.focus();
  expect(document.activeElement).toBe(button);
  lines.find(line => line.textContent === 'two')!.dispatchEvent(new MouseEvent('mouseout', { bubbles: true }));
  expect(roots.some(root => root.classList.contains('is-hovered'))).toBe(false);
});

it('honors global options and a per-block ln:false override', () => {
  create('~~~txt ln:false\nbody\n~~~', { lineNumbers: true, copyButton: false, header: false });
  expect(view.dom.querySelector('.q-code-styler-header')).toBeNull();
  expect(view.dom.querySelector('[data-ln]')).toBeNull();
});
it('copies unterminated fences through the end of the block and updates after editing', async () => {
  const write = vi.spyOn(navigator.clipboard, 'writeText').mockResolvedValue();
  create('~~~txt\nfirst');
  view.dispatch({ changes: { from: view.state.doc.length, insert: '\nsecond' } });
  view.dom.querySelector<HTMLButtonElement>(`[aria-label="${t('plugins.codeStyler.copy')}"]`)!.click();
  await Promise.resolve();
  expect(write).toHaveBeenCalledWith('first\nsecond');
});

it('shows clipboard errors and keeps fence text editable', async () => {
  vi.spyOn(navigator.clipboard, 'writeText').mockRejectedValue(new Error('denied'));
  create('```js\nbody\n```');
  view.dom.querySelector<HTMLButtonElement>(`[aria-label="${t('plugins.codeStyler.copy')}"]`)!.click();
  await Promise.resolve();
  expect(view.dom.querySelector('.q-code-styler-header [aria-live]')!.textContent).toBe(t('plugins.codeStyler.copyFailed'));
  view.dispatch({ changes: { from: 3, to: 5, insert: 'ts' } });
  expect(view.state.doc.line(1).text).toBe('```ts');
  expect(view.dom.querySelector('.q-code-styler-header')!.textContent).toContain('ts');
});

it('collapses both fences outside the whole block and reveals them anywhere inside', () => {
  const doc = 'before\n\n```js\none\ntwo\n```\n\nafter';
  create(doc);
  const lines = () => [...view.dom.querySelectorAll('.cm-line')].map(line => line.textContent);
  expect(lines()).not.toContain('```js');
  expect(lines()).not.toContain('```');
  expect(view.dom.querySelector('.q-code-styler-header')).not.toBeNull();
  for (const anchor of [doc.indexOf('```js'), doc.indexOf('one'), doc.indexOf('two'), doc.lastIndexOf('```') + 3]) {
    view.dispatch({ selection: { anchor } });
    expect(lines()).toContain('```js');
    expect(lines()).toContain('```');
  }
  view.dispatch({ selection: { anchor: doc.length } });
  expect(lines()).not.toContain('```js');
  expect(lines()).not.toContain('```');
  expect(view.state.doc.toString()).toBe(doc);
});

it('always collapses fences when caret reveal is disabled, including after reconfiguration', () => {
  const reveal = new Compartment();
  const doc = '```js\nbody\n```';
  view = new EditorView({ parent: document.body, state: EditorState.create({
    doc, selection: { anchor: 7 },
    extensions: [editorMarkdownSupport, codeBlockExtension, codeStylerExtension(settings), reveal.of(revealAtCaretFacet.of(true))],
  }) });
  expect(view.dom.textContent).toContain('```js');
  view.dispatch({ effects: reveal.reconfigure(revealAtCaretFacet.of(false)) });
  expect(view.dom.textContent).not.toContain('```');
  expect(view.dom.querySelector('.q-md-code-copy')).not.toBeNull();
});

it('numbers inserted and empty body lines immediately without numbering the closing fence', () => {
  create('``` ln:98\none\n```', { lineNumbers: true });
  const at = view.state.doc.line(3).from;
  view.dispatch({ changes: { from: at, insert: 'two\n\nthree\n' } });
  expect([...view.dom.querySelectorAll<HTMLElement>('[data-ln]')].map(line => [line.dataset.ln, line.textContent]))
    .toEqual([['98', 'one'], ['99', 'two'], ['100', ''], ['101', 'three']]);
  view.dispatch({ changes: { from: view.state.doc.line(3).from, to: view.state.doc.line(6).from } });
  expect([...view.dom.querySelectorAll<HTMLElement>('[data-ln]')].map(line => line.dataset.ln)).toEqual(['98']);
});

it('uses a compact column sized by the largest displayed number and leaves unnumbered padding intact', () => {
  create('``` ln:9\none\ntwo\n```');
  const lines = [...view.dom.querySelectorAll<HTMLElement>('[data-ln]')];
  expect(lines.map(line => line.style.getPropertyValue('--q-code-ln-width'))).toEqual(['2ch', '2ch']);
  const css = [...document.styleSheets].flatMap(sheet => [...sheet.cssRules].map(rule => rule.cssText)).join('\n');
  expect(css).toContain('calc(var(--q-block-padding) + var(--q-code-ln-width) + 1ch)');
  expect(css).toContain('width: var(--q-code-ln-width)');
  expect(css).toContain('text-align: right');
  expect(css).toContain('color: var(--q-text-secondary)');
  view.destroy();
  create('~~~ ln:false\nbody\n~~~', { lineNumbers: true });
  const unnumbered = view.dom.querySelector<HTMLElement>('.q-md-code-line')!;
  expect(unnumbered.style.getPropertyValue('--q-code-ln-width')).toBe('');
  expect(unnumbered.classList.contains('q-md-code-numbered')).toBe(false);
});

it.each([
  ['```` ln:true', '```', '~~~~', '`````'],
  ['~~~~ ln:true', '~~~', '````', '~~~~~'],
])('closes only with the same fence character and at least the opening length (%s)', (opening, short, other, closing) => {
  create([opening, 'one', short, other, 'two', closing, 'after'].join('\n'));
  expect([...view.dom.querySelectorAll<HTMLElement>('[data-ln]')].map(line => line.textContent))
    .toEqual(['one', short, other, 'two']);
});

it('numbers every inserted line immediately even while the incremental syntax tree still has its previous end', () => {
  create('``` ln:true\none\n```');
  const previous = syntaxTree(view.state);
  vi.mocked(syntaxTree).mockReturnValue(previous);
  vi.mocked(syntaxTreeAvailable).mockReturnValue(false);
  view.dispatch({ changes: { from: view.state.doc.line(3).from, insert: 'two\nthree\n' } });
  expect([...view.dom.querySelectorAll<HTMLElement>('[data-ln]')].map(line => [line.dataset.ln, line.textContent]))
    .toEqual([['1', 'one'], ['2', 'two'], ['3', 'three']]);
  expect(view.dom.querySelectorAll('.q-md-code-line.q-md-code-numbered')).toHaveLength(3);
});

it.each(['```js\n```', '```\n```', '~~~', '~~~\n', '~~~\n\n~~~'])('collapses empty and unterminated fences without losing the copy control (%s)', async doc => {
  const write = vi.spyOn(navigator.clipboard, 'writeText').mockResolvedValue();
  create(doc);
  view.dispatch({ effects: StateEffect.appendConfig.of(revealAtCaretFacet.of(false)) });
  expect(view.dom.textContent).not.toContain('```');
  expect(view.dom.textContent).not.toContain('~~~');
  expect(view.dom.querySelector('[data-ln]')).toBeNull();
  const button = view.dom.querySelector<HTMLButtonElement>('.q-md-code-copy')!;
  expect(button).not.toBeNull();
  button.click();
  await Promise.resolve();
  expect(write).toHaveBeenCalledWith(doc === '~~~\n\n~~~' ? '\n' : '');
});

it('keeps fenced-looking lines in nested markdown contexts apart from a top level block', () => {
  create('> ```js\n> nested\n> ```\n\n``` ln:7\ntop\n```');
  expect([...view.dom.querySelectorAll<HTMLElement>('[data-ln]')].map(line => [line.dataset.ln, line.textContent]))
    .toEqual([['7', 'top']]);
  expect(view.dom.querySelectorAll('.q-md-code-copy')).toHaveLength(1);
});

it('finds a fenced block beyond the incremental parser budget in a long document', () => {
  const prefix = 'paragraph\n\n'.repeat(5000);
  create(prefix + '``` ln:true\none\n```');
  const blockFrom = prefix.length;
  view.dispatch({ selection: { anchor: blockFrom + 13 } });
  view.dispatch({ changes: { from: view.state.doc.length - 3, insert: 'two\nthree\n' } });
  const numbers: string[] = [];
  for (const decorations of view.state.facet(EditorView.decorations)) {
    if (typeof decorations !== 'function') decorations.between(blockFrom, view.state.doc.length, (_from, _to, decoration) => {
      if (decoration.spec.attributes?.['data-ln']) numbers.push(decoration.spec.attributes['data-ln']);
    });
  }
  expect(numbers).toEqual(['1', '2', '3']);
});

it('joins the caption and body without styled ghost lines for either hidden fence', () => {
  const doc = 'before\n\n```js title:"demo.js"\none\ntwo\nthree\nfour\n```\n\nafter';
  create(doc);
  const styled = () => [...view.dom.querySelectorAll<HTMLElement>('.cm-line.q-md-code-line')];
  const edges = (edge: string) => styled().filter(line => line.classList.contains(`q-md-code-line--${edge}`)).map(line => line.textContent);
  expect(styled().map(line => line.textContent)).toEqual(['one', 'two', 'three', 'four']);
  expect(edges('first')).toEqual(['one']);
  expect(edges('last')).toEqual(['four']);
  const header = view.dom.querySelector<HTMLElement>('.q-code-styler-header')!;
  expect([...view.contentDOM.querySelectorAll('.cm-line')].map(line => line.textContent))
    .toEqual(['before', '', 'one', 'two', 'three', 'four', '', 'after']);
  expect(header.nextElementSibling?.classList.contains('cm-line')).toBe(false);
  expect(styled()[0].classList.contains('q-code-styler-captioned')).toBe(true);
  expect(getComputedStyle(styled()[0]).paddingTop).toBe('0px');
  expect(styled()[3].nextElementSibling?.classList.contains('q-md-code-line')).toBe(false);
  view.dispatch({ selection: { anchor: doc.indexOf('two') } });
  expect(styled().map(line => line.textContent)).toEqual(['```js title:"demo.js"', 'one', 'two', 'three', 'four', '```']);
  expect(edges('first')).toEqual(['```js title:"demo.js"']);
  expect(edges('last')).toEqual(['```']);
  view.dispatch({ selection: { anchor: doc.length } });
  expect(styled().map(line => line.textContent)).toEqual(['one', 'two', 'three', 'four']);
  expect(edges('first')).toEqual(['one']);
  expect(edges('last')).toEqual(['four']);
});

it.each(['', '\n'])('leaves no fence line after a single-line body at document end (%j)', suffix => {
  create('before\n\n```js title:"demo.js"\nbody\n```' + suffix);
  const lines = [...view.contentDOM.querySelectorAll<HTMLElement>('.cm-line')];
  expect(lines.map(line => line.textContent)).toEqual(suffix ? ['before', '', 'body', ''] : ['before', '', 'body']);
  const body = lines[2];
  expect(body.classList.contains('q-md-code-line--first')).toBe(true);
  expect(body.classList.contains('q-md-code-line--last')).toBe(true);
  expect(view.dom.querySelectorAll('.q-md-code-line--first')).toHaveLength(1);
  expect(view.dom.querySelectorAll('.q-md-code-line--last')).toHaveLength(1);
});

it('restores the base styling when Code Styler is removed and preserves indented code', () => {
  const styler = new Compartment();
  const doc = 'before\n\n```js\nbody\n```\n\n    indented';
  view = new EditorView({ parent: document.body, state: EditorState.create({
    doc, extensions: [editorMarkdownSupport, codeBlockExtension, styler.of(codeStylerExtension(settings))],
  }) });
  const styled = () => [...view.dom.querySelectorAll('.cm-line.q-md-code-line')].map(line => line.textContent);
  expect(styled()).toEqual(['body', '    indented']);
  view.dispatch({ effects: styler.reconfigure([]) });
  expect(styled()).toEqual(['```js', 'body', '```', '    indented']);
  view.dispatch({ effects: styler.reconfigure(codeStylerExtension(settings)) });
  expect(styled()).toEqual(['body', '    indented']);
});

it('keeps adjacent captions and floating controls without empty fence lines', () => {
  create('before\n\n```js title:"first.js"\none\n```\n```ts title:"second.ts"\ntwo\n```\n```\nthree\n```');
  expect([...view.contentDOM.querySelectorAll('.cm-line')].map(line => line.textContent)).toEqual(['before', '', 'one', 'two', 'three']);
  expect([...view.dom.querySelectorAll('.q-code-styler-header > span:first-child')].map(label => label.textContent)).toEqual(['first.js', 'second.ts']);
  expect(view.dom.querySelectorAll('.q-md-code-copy')).toHaveLength(3);
  expect([...view.dom.querySelectorAll('.q-md-code-line--first')].map(line => line.textContent)).toEqual(['one', 'two', 'three']);
  expect([...view.dom.querySelectorAll('.q-md-code-line--last')].map(line => line.textContent)).toEqual(['one', 'two', 'three']);
});
