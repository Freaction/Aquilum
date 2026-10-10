import { describe, expect, it } from 'vitest';
import { markdown, markdownLanguage } from '@codemirror/lang-markdown';
import { EditorSelection, EditorState, RangeSetBuilder } from '@codemirror/state';
import { Decoration } from '@codemirror/view';
import { ensureSyntaxTree, syntaxTree } from '@codemirror/language';
import { outlineMarkdownConfig } from './outline';
import { wikiLinkMarkdownConfig } from './links';
import { revealTarget } from './livePreviewReveal';
import { shouldRevealSyntax } from './livePreviewVisibility';

const MARKERS = new Set([
  'EmphasisMark', 'StrongMark', 'StrikethroughMark', 'HeaderMark',
  'LinkMark', 'URL', 'CodeMark', 'QuoteMark',
  'WikiLinkMark', 'WikiLinkAliasMark', 'HorizontalRule',
]);

function longDocWithTail(): string {
  const lines = Array.from({ length: 120 }, (_, i) => `- bullet line ${i} with filler text`);
  return `${lines.join('\n')}\n\n- [[Manifest link]]\n\n***\n`;
}

describe('ensureSyntaxTree for live preview', () => {
  it('partial syntaxTree misses bottom markers; ensure covers them', () => {
    const doc = longDocWithTail();
    const state = EditorState.create({
      doc,
      selection: EditorSelection.cursor(doc.length),
      extensions: [markdown({
        base: markdownLanguage,
        extensions: [outlineMarkdownConfig, wikiLinkMarkdownConfig],
      })],
    });

    expect(syntaxTree(state).length).toBeLessThan(doc.length);

    const tree = ensureSyntaxTree(state, doc.length, 5000);
    expect(tree).not.toBeNull();
    expect(tree!.length).toBe(doc.length);

    const manifest = doc.indexOf('[[Manifest');
    const hr = doc.indexOf('\n***\n') + 1;
    const head = doc.indexOf('\n', hr) + 1;
    const charAt = (pos: number) => state.sliceDoc(pos, pos + 1);
    const decs: { from: number; to: number; name: string }[] = [];
    const mark = Decoration.mark({ class: 'x' });

    tree!.iterate({
      enter(node) {
        if (!MARKERS.has(node.name)) return;
        const hit = revealTarget(node.node, charAt);
        if (shouldRevealSyntax(state.doc, head, hit.from, hit.to)) return;
        if (node.name === 'HorizontalRule') {
          decs.push({ from: node.from, to: node.from, name: 'HR-line' });
          decs.push({ from: node.from, to: hit.hideTo, name: node.name });
        } else {
          decs.push({ from: node.from, to: hit.hideTo, name: node.name });
        }
      },
    });

    decs.sort((a, b) => a.from - b.from || a.to - b.to);
    const builder = new RangeSetBuilder<Decoration>();
    for (const d of decs) builder.add(d.from, d.to, mark);
    expect(() => builder.finish()).not.toThrow();

    expect(decs.some((d) => d.name === 'WikiLinkMark' && d.from === manifest)).toBe(true);
    expect(decs.some((d) => d.name === 'HorizontalRule' && d.from === hr)).toBe(true);
  });
});
