import { describe, expect, it } from 'vitest';
import {
  collectBookQuoteRefs,
  formatBookQuote,
  formatReaderQuoteRef,
  parseReaderQuoteBlock,
  parseReaderQuoteHref,
  parseReaderQuoteLine,
  READER_LINK_SCHEME,
} from './bookQuotes';

describe('bookQuotes', () => {
  it('formats book quote with numbered ref link at the end', () => {
    const md = formatBookQuote('Hello  world', 'epubcfi(/6/4!/4)', undefined, 2);
    expect(md).toBe(
      `> [!quote] Hello world ${formatReaderQuoteRef(2)}(aquilum-reader:cfi=epubcfi%28%2F6%2F4%21%2F4%29)\n`,
    );
    expect(parseReaderQuoteBlock(md.trim())).toEqual({
      quoteText: 'Hello world',
      href: `${READER_LINK_SCHEME}cfi=epubcfi%28%2F6%2F4%21%2F4%29`,
      refLabel: '2',
    });
  });

  it('embeds book path in href when provided', () => {
    const md = formatBookQuote('Hello', 'epubcfi(/6/4!/4)', 'files/book.epub', 1);
    expect(md).toContain('book=files%2Fbook.epub');
    expect(parseReaderQuoteHref(
      'aquilum-reader:book=files%2Fbook.epub&cfi=epubcfi(%2F6%2F4!%2F4)',
    )).toEqual({
      bookFile: 'files/book.epub',
      cfi: 'epubcfi(/6/4!/4)',
    });
  });

  it('parses legacy inline quote links without [!quote]', () => {
    const parsed = parseReaderQuoteLine('> [Hello world](aquilum-reader:cfi=abc)');
    expect(parsed).toEqual({
      quoteText: 'Hello world',
      href: 'aquilum-reader:cfi=abc',
    });
  });

  it('escapes brackets in quote text before the ref link', () => {
    const md = formatBookQuote('[note]', 'epubcfi(/6/4!/4)', undefined, 1);
    expect(md).toBe(
      `> [!quote] \\[note\\] ${formatReaderQuoteRef(1)}(aquilum-reader:cfi=epubcfi%28%2F6%2F4%21%2F4%29)\n`,
    );
    expect(parseReaderQuoteBlock(md.trim())?.quoteText).toBe('[note]');
  });

  it('omits ref link without CFI', () => {
    expect(formatBookQuote('Hello', '')).toBe('> [!quote] Hello\n');
  });
});

describe('collectBookQuoteRefs', () => {
  it('collects cfi, label and book from quote lines', () => {
    const doc = [
      'intro',
      '> [!quote] First one [1](aquilum-reader:book=Files%2Ftest.fb2&cfi=epubcfi%28%2F6%2F4%29)',
      '',
      '> [!quote] Second one [2](aquilum-reader:book=Files%2Ftest.fb2&cfi=epubcfi%28%2F6%2F10%29)',
    ].join('\n');

    expect(collectBookQuoteRefs(doc)).toEqual([
      { cfi: 'epubcfi(/6/4)', label: '1', bookFile: 'Files/test.fb2' },
      { cfi: 'epubcfi(/6/10)', label: '2', bookFile: 'Files/test.fb2' },
    ]);
  });

  it('drops a quote once its line is gone from the note', () => {
    const kept = '> [!quote] Kept [1](aquilum-reader:cfi=abc)';
    const removed = '> [!quote] Removed [2](aquilum-reader:cfi=def)';
    expect(collectBookQuoteRefs(`${kept}\n${removed}`)).toHaveLength(2);
    expect(collectBookQuoteRefs(kept)).toEqual([
      { cfi: 'abc', label: '1', bookFile: undefined },
    ]);
  });

  it('numbers legacy quotes that carry no ref label', () => {
    const doc = '> Legacy[[→]](aquilum-reader:cfi=abc)';
    expect(collectBookQuoteRefs(doc)).toEqual([
      { cfi: 'abc', label: '1', bookFile: undefined },
    ]);
  });
});
