import { describe, expect, it } from 'vitest';
import { markdown, markdownLanguage } from '@codemirror/lang-markdown';
import { EditorSelection, EditorState } from '@codemirror/state';
import { syntaxTree } from '@codemirror/language';
import { outlineMarkdownConfig } from './outline';
import { wikiLinkMarkdownConfig } from './links';
import { editorMarkdownExtensions } from './markdownConfig';
import {
  collectCollapseRanges,
  mergeCollapseRanges,
} from './livePreviewPlugin';

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

describe('mergeCollapseRanges', () => {
  it('merges adjacent ] ( url ) into one range', () => {
    expect(mergeCollapseRanges([
      { from: 17, to: 18 },
      { from: 18, to: 19 },
      { from: 19, to: 58 },
      { from: 58, to: 59 },
      { from: 6, to: 7 },
    ])).toEqual([
      { from: 6, to: 7 },
      { from: 17, to: 59 },
    ]);
  });
});

describe('collectCollapseRanges', () => {
  const doc = 'see [hello world](https://example.com/very/long/path) end\nnext\n';
  const open = doc.indexOf('[');
  const close = doc.indexOf(']');
  const afterUrl = doc.indexOf(' end');
  const label = doc.indexOf('hello');

  it('collapses [ and ](url) when caret is off the link', () => {
    const state = createState(doc, doc.indexOf('next'));
    expect(collectCollapseRanges(state, syntaxTree(state))).toEqual([
      { from: open, to: open + 1 },
      { from: close, to: afterUrl },
    ]);
  });

  it('does not collapse when caret is inside the link', () => {
    const state = createState(doc, label);
    expect(collectCollapseRanges(state, syntaxTree(state))).toEqual([]);
  });

  it('collapses wiki target and marks when alias exists and caret is off the wiki', () => {
    const wikiDoc = 'before [[Target|Alias]] after\n';
    const state = createState(wikiDoc, wikiDoc.indexOf('after'));
    expect(collectCollapseRanges(state, syntaxTree(state))).toEqual([
      { from: wikiDoc.indexOf('[['), to: wikiDoc.indexOf('|') + 1 },
      { from: wikiDoc.indexOf(']]'), to: wikiDoc.indexOf(']]') + 2 },
    ]);
  });

  it('reveals full [[target|alias]] when caret is inside the wiki link', () => {
    const wikiDoc = 'before [[Target|Alias]] after\n';
    const state = createState(wikiDoc, wikiDoc.indexOf('Target') + 2);
    expect(collectCollapseRanges(state, syntaxTree(state))).toEqual([]);
  });

  it('shows only alias text in preview (target range collapsed)', () => {
    const wikiDoc = 'text [[совесть|совести]] end\n';
    const state = createState(wikiDoc, 0);
    const collapsed = collectCollapseRanges(state, syntaxTree(state));
    const aliasFrom = wikiDoc.indexOf('совести');
    const aliasTo = aliasFrom + 'совести'.length;
    expect(collapsed.some((range) => range.from <= aliasFrom && range.to >= aliasTo)).toBe(false);
    expect(collapsed.some((range) => range.from <= wikiDoc.indexOf('совесть') && range.to > wikiDoc.indexOf('совесть'))).toBe(true);
  });

  it('keeps reader ref collapsed while caret is in quote text', () => {
    const doc = '> [!quote] Quote text [1](aquilum-reader:cfi=abc&book=files%2Flong%2Fpath)\n';
    const state = createState(doc, doc.indexOf('Quote') + 2);
    const collapsed = collectCollapseRanges(state, syntaxTree(state));
    const open = doc.indexOf('[1]');
    expect(collapsed.some((range) => range.from <= open && range.to > open)).toBe(true);
  });
});

describe('links the reader actually sees', () => {
  const fullState = (doc: string, head: number) => EditorState.create({
    doc,
    selection: EditorSelection.cursor(head),
    extensions: [
      markdown({ base: markdownLanguage, extensions: [...editorMarkdownExtensions] }),
    ],
  });

  const shown = (doc: string, head: number): string => {
    const state = fullState(doc, head);
    let out = '';
    let at = 0;
    for (const range of collectCollapseRanges(state, syntaxTree(state))) {
      out += doc.slice(at, range.from);
      at = range.to;
    }
    return out + doc.slice(at);
  };

  const frontmatter = [
    '---',
    'tags: [мозг]',
    'source: [www.youtube.com](https://www.youtube.com/watch?v=uKhsbQcFVwQ)',
    '---',
    '',
    'хвост',
  ].join('\n');

  it('keeps a title that happens to look like a domain', () => {
    const doc = 'см [www.youtube.com](https://www.youtube.com/watch?v=abc) конец\nдалее\n';
    expect(shown(doc, doc.indexOf('далее'))).toBe('см www.youtube.com конец\nдалее\n');
  });

  it('leaves a bare address in running text alone', () => {
    const doc = 'голый https://example.com/a?b=1 в тексте\nдалее\n';
    expect(shown(doc, doc.indexOf('далее'))).toBe(doc);
  });

  it('collapses a finished link inside frontmatter', () => {
    expect(shown(frontmatter, frontmatter.indexOf('хвост')))
      .toContain('source: www.youtube.com\n');
  });

  it('never eats the brackets of a yaml list in frontmatter', () => {
    expect(shown(frontmatter, frontmatter.indexOf('хвост')))
      .toContain('tags: [мозг]');
  });

  it('reveals the frontmatter link for editing when the caret is inside it', () => {
    expect(shown(frontmatter, frontmatter.indexOf('youtube') + 2)).toBe(frontmatter);
  });
});
