import { describe, expect, it } from 'vitest';
import { markdown, markdownLanguage } from '@codemirror/lang-markdown';
import { EditorSelection, EditorState, RangeSetBuilder, Text } from '@codemirror/state';
import { Decoration } from '@codemirror/view';
import { syntaxTree } from '@codemirror/language';
import { outlineMarkdownConfig } from './outline';
import { wikiLinkMarkdownConfig } from './links';
import { revealTarget } from './livePreviewReveal';
import { shouldRevealSyntax } from './livePreviewVisibility';

const MARKERS = new Set([
  'EmphasisMark', 'StrongMark', 'StrikethroughMark', 'HeaderMark',
  'LinkMark', 'URL', 'CodeMark', 'QuoteMark',
  'WikiLinkMark', 'WikiLinkAliasMark', 'HorizontalRule',
]);

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

function buildHideDecs(state: EditorState) {
  const decs: { from: number; to: number }[] = [];
  const head = state.selection.main.head;
  const charAt = (pos: number) => state.sliceDoc(pos, pos + 1);

  syntaxTree(state).iterate({
    enter(node) {
      if (!MARKERS.has(node.name)) return;
      const hit = revealTarget(node.node, charAt);
      if (shouldRevealSyntax(state.doc, head, hit.from, hit.to)) return;
      if (node.name === 'HorizontalRule') {
        decs.push({ from: node.from, to: node.from });
        decs.push({ from: node.from, to: hit.hideTo });
      } else {
        decs.push({ from: node.from, to: hit.hideTo });
      }
    },
  });

  decs.sort((a, b) => a.from - b.from || a.to - b.to);
  const builder = new RangeSetBuilder<Decoration>();
  const hidden = Decoration.mark({ class: 'q-md-hidden-syntax' });
  const transparent = Decoration.mark({ class: 'q-md-transparent-syntax' });
  const hrLine = Decoration.line({ class: 'q-md-hr-line-cm' });
  for (const d of decs) {
    builder.add(
      d.from,
      d.to,
      d.from === d.to ? hrLine : (d.to - d.from >= 3 ? transparent : hidden),
    );
  }
  return decs;
}

describe('decoration build after nested list + HR', () => {
  const doc = [
    '- [[Манифест античного человека XXI века]]',
    '',
    '1. ывавыва',
    '\t1. ',
    '',
    '***',
  ].join('\n');

  it('hides wiki marks and HR when caret is on blank line above HR', () => {
    const caret = doc.indexOf('\n***');
    const state = createState(doc, caret);
    const decs = buildHideDecs(state);
    expect(decs.some((d) => d.from === 2)).toBe(true);
    expect(decs.some((d) => d.from === doc.indexOf('***'))).toBe(true);
  });

  it('wide stuck selection still only uses head — marks off head line stay hidden', () => {
    const blank = doc.indexOf('\n***');
    const hr = doc.indexOf('***');
    const state = EditorState.create({
      doc,
      selection: EditorSelection.single(0, blank),
      extensions: [
        markdown({
          base: markdownLanguage,
          extensions: [outlineMarkdownConfig, wikiLinkMarkdownConfig],
        }),
      ],
    });
    expect(state.selection.main.head).toBe(blank);
    const decs = buildHideDecs(state);
    expect(decs.some((d) => d.from === 2)).toBe(true);
    expect(decs.some((d) => d.from === hr)).toBe(true);
  });

  it('caret on wiki line reveals wiki marks but not HR on another line', () => {
    const state = createState(doc, 2);
    const decs = buildHideDecs(state);
    expect(decs.some((d) => d.from === 2)).toBe(false);
    expect(decs.some((d) => d.from === doc.indexOf('***'))).toBe(true);
  });

  it('line gate: caret on list line never opens wiki or HR', () => {
    const listLine = Text.of(doc.split('\n')).line(3).from;
    const state = createState(doc, listLine);
    const decs = buildHideDecs(state);
    expect(decs.some((d) => d.from === 2)).toBe(true);
    expect(decs.some((d) => d.from === doc.indexOf('***'))).toBe(true);
  });
});
