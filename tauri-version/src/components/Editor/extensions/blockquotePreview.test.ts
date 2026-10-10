import { describe, expect, it } from 'vitest';
import { markdown, markdownLanguage } from '@codemirror/lang-markdown';
import { EditorState } from '@codemirror/state';
import { ensureSyntaxTree } from '@codemirror/language';
import { blockquoteStyles, collectBlockquoteDecorations } from './blockquotePreview';
import { syntaxMarkerPresentation } from './livePreviewSyntax';
import { exitEmptyBlockquote } from './outline/commands';

function quoteDecorations(doc: string, caret = 0) {
  const state = EditorState.create({
    doc,
    selection: { anchor: caret },
    extensions: [markdown({ base: markdownLanguage })],
  });
  const tree = ensureSyntaxTree(state, state.doc.length, 5000)!;
  const decorations = collectBlockquoteDecorations(
    state,
    tree,
    [{ from: 0, to: state.doc.length }],
  );
  return { state, decorations };
}

function bars(doc: string, caret = 0) {
  const { state, decorations } = quoteDecorations(doc, caret);
  const items = decorations.filter((d) => String(d.dec.spec.class).startsWith('q-md-blockquote'));
  return { state, items };
}

function linesWithBar(doc: string, caret = 0): number[] {
  const { state, items } = bars(doc, caret);
  return items.map((d) => state.doc.lineAt(d.from).number);
}

function barClasses(doc: string, caret = 0): string[] {
  return bars(doc, caret).items.map((d) => d.dec.spec.class as string);
}

function calloutMarkers(doc: string, caret = 0): string[] {
  const { state, decorations } = quoteDecorations(doc, caret);
  return decorations
    .filter((d) => d.dec.spec.class === 'q-md-syntax-char')
    .map((d) => state.doc.sliceString(d.from, d.to));
}

function runExit(doc: string, caret: number) {
  let next: EditorState | null = null;
  const state = EditorState.create({
    doc,
    selection: { anchor: caret },
    extensions: [markdown({ base: markdownLanguage })],
  });
  const ok = exitEmptyBlockquote({
    state,
    dispatch: (tr) => { next = tr.state; },
  });
  const result = next as EditorState | null;
  return {
    ok,
    doc: result?.doc.toString() ?? null,
    caret: result?.selection.main.head ?? null,
  };
}

describe('blockquote preview', () => {
  it('bars plain > lines, not books or lazy continuations', () => {
    expect(linesWithBar('> hello')).toEqual([1]);
    expect(linesWithBar('> a\n> b')).toEqual([1, 2]);
    expect(linesWithBar('> hello\nlazy')).toEqual([1]);
    expect(linesWithBar('> [!book] Title\n> Автор: A')).toEqual([]);
    expect(linesWithBar('> [!book] Title', 4)).toEqual([]);
  });

  it('hides bar under mounted [!quote] widget, shows while editing', () => {
    const doc = '> [!quote] Quote [1](aquilum-reader:cfi=x)\nafter';
    expect(linesWithBar(doc, doc.indexOf('after'))).toEqual([]);
    expect(linesWithBar(doc, doc.indexOf('Quote'))).toEqual([1]);
  });

  it('joins consecutive > lines into one unbroken bar', () => {
    expect(barClasses('> a\n> b\n> c')).toEqual([
      'q-md-blockquote q-md-blockquote--joined-down',
      'q-md-blockquote q-md-blockquote--joined-up q-md-blockquote--joined-down',
      'q-md-blockquote q-md-blockquote--joined-up',
    ]);
  });

  it('keeps a lone > line rounded on both ends', () => {
    expect(barClasses('> a')).toEqual(['q-md-blockquote']);
    expect(barClasses('> a\nlazy')).toEqual(['q-md-blockquote']);
  });
});

describe('callout markers', () => {
  it('greys every [!name] marker, whatever the callout is', () => {
    expect(calloutMarkers('> [!quote] text [1](aquilum-reader:cfi=x)', 5)).toEqual(['[!quote]']);
    expect(calloutMarkers('> [!book] Title\n> Автор: A')).toEqual(['[!book]']);
    expect(calloutMarkers('> [!warning] Careful')).toEqual(['[!warning]']);
  });

  it('leaves plain quotes alone', () => {
    expect(calloutMarkers('> plain quote')).toEqual([]);
  });

  it('costs nothing while the caret is outside the callout', () => {
    const doc = '> [!quote] text [1](aquilum-reader:cfi=x)\nafter';
    expect(calloutMarkers(doc, doc.indexOf('after'))).toEqual([]);
    expect(calloutMarkers('> [!book] Title\n\nafter', '> [!book] Title\n\n'.length)).toEqual([]);
  });
});

describe('blockquote indent', () => {
  it('keeps quote text at the same x with and without the caret', () => {
    expect(blockquoteStyles['.cm-line.q-md-blockquote'].paddingLeft)
      .toBe(blockquoteStyles['.q-md-blockquote'].paddingLeft);
  });

  it('hangs the raw > back out of the padding', () => {
    expect(blockquoteStyles['.cm-line.q-md-blockquote'].textIndent).toBe('-2ch');
    expect(blockquoteStyles['.q-md-blockquote']['--q-md-quote-indent']).toContain('2ch');
  });

  it('hides the > marker without collapsing its width', () => {
    expect(syntaxMarkerPresentation('QuoteMark')).toBe('transparent');
    expect(syntaxMarkerPresentation('EmphasisMark')).toBe('hidden');
  });
});

describe('exitEmptyBlockquote', () => {
  it('exits with a blank line between quote and new paragraph', () => {
    const doc = '> text\n> ';
    const { ok, doc: result, caret } = runExit(doc, doc.length);
    expect(ok).toBe(true);
    expect(result).toBe('> text\n\n');
    expect(caret).toBe('> text\n\n'.length);
  });

  it('ignores non-empty quotes and plain text', () => {
    expect(runExit('> text', 6).ok).toBe(false);
    expect(runExit('hello', 5).ok).toBe(false);
  });
});
