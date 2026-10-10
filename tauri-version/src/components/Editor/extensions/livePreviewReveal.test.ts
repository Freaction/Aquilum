import { describe, expect, it } from 'vitest';
import { markdown, markdownLanguage } from '@codemirror/lang-markdown';
import { EditorState, Text } from '@codemirror/state';
import { syntaxTree } from '@codemirror/language';
import { outlineMarkdownConfig } from './outline';
import { wikiLinkMarkdownConfig } from './links';
import { revealTarget } from './livePreviewReveal';
import { shouldRevealSyntax } from './livePreviewVisibility';

function parse(doc: string) {
  return EditorState.create({
    doc,
    extensions: [markdown({
      base: markdownLanguage,
      extensions: [outlineMarkdownConfig, wikiLinkMarkdownConfig],
    })],
  });
}

function isMarkerRevealed(doc: string, caret: number, markName: string, index = 0): boolean {
  const state = parse(doc);
  const charAt = (pos: number) => state.sliceDoc(pos, pos + 1);
  let seen = 0;
  let revealed = false;
  syntaxTree(state).iterate({
    enter(node) {
      if (node.name !== markName) return;
      if (seen++ !== index) return;
      const hit = revealTarget(node.node, charAt);
      revealed = shouldRevealSyntax(state.doc, caret, hit.from, hit.to);
    },
  });
  return revealed;
}

describe('revealTarget + shouldRevealSyntax', () => {
  it('reveals bare *** (HR) with caret on either edge', () => {
    const doc = '***';
    expect(isMarkerRevealed(doc, 0, 'HorizontalRule')).toBe(true);
    expect(isMarkerRevealed(doc, 3, 'HorizontalRule')).toBe(true);
  });

  it('reveals both sides of emphasis while caret is inside its text', () => {
    const doc = '***bold***';
    expect(isMarkerRevealed(doc, 5, 'EmphasisMark', 0)).toBe(true);
    expect(isMarkerRevealed(doc, 5, 'EmphasisMark', 1)).toBe(true);
    expect(isMarkerRevealed(doc, 5, 'EmphasisMark', 2)).toBe(true);
    expect(isMarkerRevealed(doc, 5, 'EmphasisMark', 3)).toBe(true);
    expect(isMarkerRevealed(doc, 0, 'EmphasisMark', 0)).toBe(true);
    expect(isMarkerRevealed(doc, 10, 'EmphasisMark', 3)).toBe(true);
  });

  it('reveals both italic markers while caret is inside its text', () => {
    const doc = '*italic*';
    expect(isMarkerRevealed(doc, 4, 'EmphasisMark', 0)).toBe(true);
    expect(isMarkerRevealed(doc, 4, 'EmphasisMark', 1)).toBe(true);
  });

  it('reveals both strong markers while caret is inside its text', () => {
    const doc = '**text**';
    expect(syntaxTree(parse(doc)).toString()).toContain('StrongEmphasis');
    expect(isMarkerRevealed(doc, 4, 'EmphasisMark', 0)).toBe(true);
    expect(isMarkerRevealed(doc, 4, 'EmphasisMark', 1)).toBe(true);
  });

  it('does not reveal wiki when caret is on HR below', () => {
    const doc = '[[link]]\n***';
    const state = parse(doc);
    const charAt = (pos: number) => state.sliceDoc(pos, pos + 1);
    const hrCaret = doc.length;
    let wikiActive = false;
    syntaxTree(state).iterate({
      enter(node) {
        if (node.name !== 'WikiLinkMark') return;
        const hit = revealTarget(node.node, charAt);
        if (shouldRevealSyntax(state.doc, hrCaret, hit.from, hit.to)) {
          wikiActive = true;
        }
      },
    });
    expect(wikiActive).toBe(false);
    expect(isMarkerRevealed(doc, hrCaret, 'HorizontalRule')).toBe(true);
  });

  it('reveals wiki from first char through caret-after-last-char on its line', () => {
    const doc = Text.of(['[[link]]', '***']);
    expect(shouldRevealSyntax(doc, 0, 0, 8)).toBe(true);
    expect(shouldRevealSyntax(doc, 3, 0, 8)).toBe(true);
    expect(shouldRevealSyntax(doc, 8, 0, 8)).toBe(true);
    expect(shouldRevealSyntax(doc, 9, 0, 8)).toBe(false);
  });

  it('reveals HeaderMark from anywhere on the heading line', () => {
    const doc = '# Заголовок первого уровня';
    for (let caret = 0; caret <= doc.length; caret += 1) {
      expect(isMarkerRevealed(doc, caret, 'HeaderMark')).toBe(true);
    }
  });

  it('reveals HeaderMark on every heading level', () => {
    for (const prefix of ['#', '##', '###', '####', '#####', '######']) {
      const doc = `${prefix} Заголовок`;
      expect(isMarkerRevealed(doc, doc.length, 'HeaderMark')).toBe(true);
      expect(isMarkerRevealed(doc, 0, 'HeaderMark')).toBe(true);
    }
  });

  it('hides HeaderMark together with its trailing space', () => {
    const state = parse('## Текст');
    const charAt = (pos: number) => state.sliceDoc(pos, pos + 1);
    let hideTo = -1;
    syntaxTree(state).iterate({
      enter(node) {
        if (node.name === 'HeaderMark') hideTo = revealTarget(node.node, charAt).hideTo;
      },
    });
    expect(hideTo).toBe(3);
  });

  it('does not reveal a heading mark from another line', () => {
    const doc = '# Заголовок\nтекст абзаца';
    expect(isMarkerRevealed(doc, doc.length, 'HeaderMark')).toBe(false);
  });

  it('reveals both marks of a closed ATX heading', () => {
    const doc = '## Заголовок ##';
    expect(isMarkerRevealed(doc, 5, 'HeaderMark', 0)).toBe(true);
    expect(isMarkerRevealed(doc, 5, 'HeaderMark', 1)).toBe(true);
  });

  it('reveals QuoteMark for the active blockquote only', () => {
    expect(isMarkerRevealed('> hello', 2, 'QuoteMark')).toBe(true);
    expect(isMarkerRevealed('> hello', 6, 'QuoteMark')).toBe(true);
    const doc = '> a\n> b\n\n> c';
    expect(isMarkerRevealed(doc, doc.indexOf('b'), 'QuoteMark', 0)).toBe(true);
    expect(isMarkerRevealed(doc, doc.indexOf('b'), 'QuoteMark', 2)).toBe(false);
    expect(isMarkerRevealed(doc, doc.indexOf('c'), 'QuoteMark', 2)).toBe(true);
  });
});
