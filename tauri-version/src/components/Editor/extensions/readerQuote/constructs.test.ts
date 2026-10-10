import { describe, expect, it } from 'vitest';
import { EditorState, Text } from '@codemirror/state';
import { editablePreviewBlockEntryPos } from '../livePreviewEditableSpans';
import { findReaderQuotes, readerQuoteEditEntryPos, readerQuoteTextRange } from './constructs';

const USER_QUOTE = '> [!quote] Вот примерно таким было бы наше восприятие мира всегда, если бы не ретикулярная формация. В данном примере она уже пробудила вас, но ещё не включилась на полную мощь и не успела выполнить свою интегрирующую функцию. [3](aquilum-reader:book=Files%2Ftest.fb2&cfi=epubcfi%28%2F6%2F10%29)';

describe('readerQuote constructs', () => {
  it('finds [!quote] callout lines', () => {
    const doc = Text.of([
      'intro',
      '> [!quote] Quote text [1](aquilum-reader:cfi=abc)',
      'after',
    ]);
    const spans = findReaderQuotes(doc);
    expect(spans).toHaveLength(1);
    expect(spans[0]?.quoteText).toBe('Quote text');
    expect(spans[0]?.href).toBe('aquilum-reader:cfi=abc');
    expect(spans[0]?.refLabel).toBe('1');
  });

  it('finds legacy reader quote lines', () => {
    const doc = Text.of(['> Quote text[[→]](aquilum-reader:cfi=abc)']);
    expect(findReaderQuotes(doc)).toHaveLength(1);
  });
});

describe('readerQuoteEditEntryPos', () => {
  it('lands before [N] ref, not inside url', () => {
    const doc = Text.of([USER_QUOTE, 'after']);
    const span = findReaderQuotes(doc)[0]!;
    const entry = readerQuoteEditEntryPos(span);
    expect(doc.sliceString(entry, entry + 1)).toBe('.');
    expect(entry).toBeLessThan(span.to - 20);
  });

  it('uses edit entry when entering quote from below', () => {
    const doc = Text.of(['before', USER_QUOTE, 'after']);
    const state = EditorState.create({ doc, selection: { anchor: doc.line(3).from } });
    const pos = editablePreviewBlockEntryPos(state, 3, false);
    const span = findReaderQuotes(doc)[0]!;
    expect(pos).toBe(readerQuoteEditEntryPos(span));
  });
});

describe('readerQuoteTextRange', () => {
  it('covers the quote text without header and ref link', () => {
    const doc = Text.of([USER_QUOTE, 'after']);
    const span = findReaderQuotes(doc)[0]!;
    const range = readerQuoteTextRange(span);
    expect(doc.sliceString(range.from, range.to)).toBe(span.quoteText);
  });

  it('covers legacy quote text without the arrow link', () => {
    const doc = Text.of(['> Quote text[[→]](aquilum-reader:cfi=abc)']);
    const span = findReaderQuotes(doc)[0]!;
    const range = readerQuoteTextRange(span);
    expect(doc.sliceString(range.from, range.to)).toBe('Quote text');
  });
});
