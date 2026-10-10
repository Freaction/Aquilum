import { describe, expect, it } from 'vitest';
import {
  resolveReaderOpenTarget,
  setReaderState,
} from './readerState';

describe('resolveReaderOpenTarget', () => {
  it('prefers explicit initial CFI', () => {
    const book = `book-${crypto.randomUUID()}.epub`;
    setReaderState(book, 5, 'epubcfi(/6/4!/4)');
    expect(resolveReaderOpenTarget(book, { current: 5, total: 10 }, 'epubcfi(/6/2!/2)'))
      .toBe('epubcfi(/6/2!/2)');
  });

  it('uses cached CFI when current matches FM pages', () => {
    const book = `book-${crypto.randomUUID()}.epub`;
    setReaderState(book, 5, 'epubcfi(/6/4!/4)');
    expect(resolveReaderOpenTarget(book, { current: 5, total: 10 }))
      .toBe('epubcfi(/6/4!/4)');
  });

  it('falls back to FM fraction and clears stale CFI when current differs', () => {
    const book = `book-${crypto.randomUUID()}.epub`;
    setReaderState(book, 5, 'epubcfi(/6/4!/4)');
    expect(resolveReaderOpenTarget(book, { current: 8, total: 10 }))
      .toEqual({ fraction: 0.8 });
    expect(resolveReaderOpenTarget(book, { current: 5, total: 10 }))
      .toEqual({ fraction: 0.5 });
  });
});
