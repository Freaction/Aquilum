import { describe, expect, it } from 'vitest';
import { EditorState, Text } from '@codemirror/state';
import { findBookCallouts } from './bookCallout/constructs';
import { findReaderQuotes, readerQuoteEditEntryPos } from './readerQuote/constructs';
import {
  editablePreviewBlockEntryPos,
  selectionIntersectsEditablePreview,
  selectionInEditablePreviewBlock,
  shouldRevealEditablePreviewMarker,
} from './livePreviewEditableSpans';

describe('selectionIntersectsEditablePreview', () => {
  it('is true when caret touches book callout edges', () => {
    const doc = Text.of(['> [!book] [[Новая книга]]', 'after']);
    const span = findBookCallouts(doc)[0]!;
    expect(selectionIntersectsEditablePreview(
      EditorState.create({ doc, selection: { anchor: span.from } }),
      span,
    )).toBe(true);
    expect(selectionIntersectsEditablePreview(
      EditorState.create({ doc, selection: { anchor: span.to } }),
      span,
    )).toBe(true);
    expect(selectionIntersectsEditablePreview(
      EditorState.create({ doc, selection: { anchor: doc.line(2).from } }),
      span,
    )).toBe(false);
  });

  it('is true while caret is inside a reader quote', () => {
    const doc = Text.of(['> [!quote] Quote text [1](aquilum-reader:cfi=abc)']);
    const span = findReaderQuotes(doc)[0]!;
    const state = EditorState.create({ doc, selection: { anchor: span.from + 2 } });
    expect(selectionIntersectsEditablePreview(state, span)).toBe(true);
  });

  it('detects caret inside any preview block', () => {
    const doc = Text.of(['> [!quote] Quote text [1](aquilum-reader:cfi=abc)', 'after']);
    const span = findReaderQuotes(doc)[0]!;
    const inside = EditorState.create({ doc, selection: { anchor: span.to - 2 } });
    const outside = EditorState.create({ doc, selection: { anchor: doc.line(2).from } });
    expect(selectionInEditablePreviewBlock(inside)).toBe(true);
    expect(selectionInEditablePreviewBlock(outside)).toBe(false);
  });
});

describe('shouldRevealEditablePreviewMarker', () => {
  it('reveals quote marks while caret is anywhere in the block', () => {
    const doc = Text.of(['> [!book] Чистый код', '> Автор: A', '', 'after']);
    const spans = findBookCallouts(doc);
    expect(shouldRevealEditablePreviewMarker(spans, 5, 0)).toBe(true);
    expect(shouldRevealEditablePreviewMarker(spans, 5, doc.line(2).from)).toBe(true);
    expect(shouldRevealEditablePreviewMarker(spans, 0, 0)).toBe(true);
    expect(shouldRevealEditablePreviewMarker(spans, doc.line(4).from, 0)).toBe(false);
  });

  it('reveals reader quote markers while caret is inside', () => {
    const doc = Text.of(['> [!quote] Quote text [1](aquilum-reader:cfi=abc)']);
    const span = findReaderQuotes(doc)[0]!;
    expect(shouldRevealEditablePreviewMarker([span], span.from + 2, span.from)).toBe(true);
    expect(shouldRevealEditablePreviewMarker([span], span.from + 2, span.from, 'QuoteMark')).toBe(true);
    expect(shouldRevealEditablePreviewMarker([span], span.from + 2, span.from, 'LinkMark')).toBe(false);
    expect(shouldRevealEditablePreviewMarker([span], span.from + 2, span.from, 'URL')).toBe(false);
  });
});

describe('editablePreviewBlockEntryPos', () => {
  const quoteDoc = Text.of([
    'before',
    '> [!quote] Quote text[[→]](aquilum-reader:cfi=abc)',
    'after',
  ]);

  it('enters quote at end of text when moving up from below', () => {
    const state = EditorState.create({ doc: quoteDoc, selection: { anchor: quoteDoc.line(3).from } });
    const pos = editablePreviewBlockEntryPos(state, 3, false);
    const span = findReaderQuotes(quoteDoc)[0]!;
    expect(pos).toBe(readerQuoteEditEntryPos(span));
  });

  it('enters quote at start when moving down from above', () => {
    const state = EditorState.create({ doc: quoteDoc, selection: { anchor: quoteDoc.line(1).to } });
    const pos = editablePreviewBlockEntryPos(state, 1, true);
    const span = findReaderQuotes(quoteDoc)[0]!;
    expect(pos).toBe(span.from);
  });

  it('enters multi-line book callout at last line when moving up from below', () => {
    const doc = Text.of([
      'before',
      '> [!book] Title',
      '> Автор: A',
      'after',
    ]);
    const state = EditorState.create({ doc, selection: { anchor: doc.line(4).from } });
    const pos = editablePreviewBlockEntryPos(state, 4, false);
    const span = findBookCallouts(doc)[0]!;
    expect(pos).toBe(span.to);
  });

  it('returns null when already editing the block', () => {
    const span = findReaderQuotes(quoteDoc)[0]!;
    const state = EditorState.create({ doc: quoteDoc, selection: { anchor: span.from + 1 } });
    expect(editablePreviewBlockEntryPos(state, 3, false)).toBeNull();
  });

  it('returns null when caret is not adjacent to the block', () => {
    const state = EditorState.create({ doc: quoteDoc, selection: { anchor: quoteDoc.line(1).from } });
    expect(editablePreviewBlockEntryPos(state, 1, false)).toBeNull();
  });
});
