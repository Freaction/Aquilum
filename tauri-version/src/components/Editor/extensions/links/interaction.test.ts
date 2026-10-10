import { describe, expect, it } from 'vitest';
import { markdown, markdownLanguage } from '@codemirror/lang-markdown';
import { EditorSelection, EditorState } from '@codemirror/state';
import { outlineMarkdownConfig } from '../outline';
import { readerRefLinkClickAllowed } from './interaction';
import { wikiLinkMarkdownConfig } from './syntax';

const USER_QUOTE = '> [!quote] Вот примерно таким было бы наше восприятие мира всегда, если бы не ретикулярная формация. В данном примере она уже пробудила вас, но ещё не включилась на полную мощь и не успела выполнить свою интегрирующую функцию. [3](aquilum-reader:book=Files%2Ftest.fb2&cfi=epubcfi%28%2F6%2F10%29)';

function createState(doc: string, head: number) {
  return EditorState.create({
    doc,
    selection: EditorSelection.cursor(head),
    extensions: [
      markdown({
        base: markdownLanguage,
        extensions: [outlineMarkdownConfig, wikiLinkMarkdownConfig],
      }),
    ],
  });
}

describe('readerRefLinkClickAllowed', () => {
  const doc = '> [!quote] Quote text [1](aquilum-reader:cfi=abc&book=files%2Flong%2Fpath)\nafter\n';
  const refStart = doc.indexOf('[1]');
  const urlMid = doc.indexOf('aquilum-reader') + 10;
  const quoteMid = doc.indexOf('Quote') + 2;
  const url = 'aquilum-reader:cfi=abc';

  it('blocks click on ref while caret is in quote text', () => {
    const state = createState(doc, quoteMid);
    expect(readerRefLinkClickAllowed(state, refStart, url)).toBe(false);
  });

  it('blocks click on url while caret is in quote text', () => {
    const state = createState(doc, quoteMid);
    expect(readerRefLinkClickAllowed(state, urlMid, url)).toBe(false);
  });

  it('allows click on ref only when caret is on the label', () => {
    const state = createState(doc, refStart + 1);
    expect(readerRefLinkClickAllowed(state, refStart + 1, url)).toBe(true);
  });

  it('allows normal external links', () => {
    const plain = 'see [web](https://example.com)\n';
    const state = createState(plain, plain.indexOf('web'));
    expect(readerRefLinkClickAllowed(state, plain.indexOf('web'), 'https://example.com')).toBe(true);
  });

  it('blocks reader ref in long quote while caret is in text', () => {
    const mid = USER_QUOTE.indexOf('ретикулярная') + 4;
    const ref = USER_QUOTE.indexOf('[3]') + 1;
    const state = createState(`${USER_QUOTE}\nafter`, mid);
    expect(readerRefLinkClickAllowed(state, ref, 'aquilum-reader:book=Files%2Ftest.fb2&cfi=x')).toBe(false);
  });
});
