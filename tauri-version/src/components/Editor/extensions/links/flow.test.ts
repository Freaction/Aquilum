import { describe, expect, it } from 'vitest';
import { markdown, markdownLanguage } from '@codemirror/lang-markdown';
import { EditorState } from '@codemirror/state';
import { syntaxTree } from '@codemirror/language';
import { linkLabelRange, linkUrlChromeRange } from './flow';

function labelOf(doc: string): { from: number; to: number; text: string } | null {
  const state = EditorState.create({
    doc,
    extensions: [markdown({ base: markdownLanguage })],
  });
  let found: { from: number; to: number; text: string } | null = null;
  syntaxTree(state).iterate({
    enter(node) {
      if (found || (node.name !== 'Link' && node.name !== 'Image')) return;
      const label = linkLabelRange(node.node);
      if (!label) return;
      found = { ...label, text: doc.slice(label.from, label.to) };
    },
  });
  return found;
}

describe('linkLabelRange', () => {
  it('covers only the visible label, not ](url)', () => {
    const doc = 'see [hello world](https://example.com/very/long/path) end';
    const label = labelOf(doc);
    expect(label?.text).toBe('hello world');
    expect(doc.slice(label!.to, label!.to + 2)).toBe('](');
  });

  it('covers image alt text only', () => {
    const doc = '![alt text](https://cdn.example/a.png)';
    expect(labelOf(doc)?.text).toBe('alt text');
  });
});

describe('linkUrlChromeRange', () => {
  it('covers ](url) from closing bracket through paren', () => {
    const doc = 'see [hello world](https://example.com/very/long/path) end';
    const state = EditorState.create({
      doc,
      extensions: [markdown({ base: markdownLanguage })],
    });
    let chrome: { from: number; to: number } | null = null;
    syntaxTree(state).iterate({
      enter(node) {
        if (chrome || node.name !== 'Link') return;
        chrome = linkUrlChromeRange(node.node);
      },
    });
    expect(doc.slice(chrome!.from, chrome!.to)).toBe('](https://example.com/very/long/path)');
  });
});
