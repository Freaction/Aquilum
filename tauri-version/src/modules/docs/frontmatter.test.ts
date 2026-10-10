import { describe, expect, it } from 'vitest';
import {
  FM_BOOK_COVER,
  FM_BOOK_FILE,
  FM_PAGE_COVER,
  FM_READ_PERCENT,
  FM_READER_POSITION,
  formatReadPercent,
  parseFrontmatter,
  parseReadPercent,
  resolveBookFields,
  setFrontmatterField,
} from './frontmatter';

describe('frontmatter book fields', () => {
  it('reads canonical and legacy keys', () => {
    const fields = resolveBookFields({
      [FM_BOOK_COVER]: 'Files/cover.jpg',
      [FM_PAGE_COVER]: 'Files/page.jpg',
      [FM_BOOK_FILE]: 'Files/dune.epub',
      [FM_READER_POSITION]: 'epubcfi(/6/4!/4)',
      [FM_READ_PERCENT]: '42',
    });
    expect(fields).toEqual({
      bookCoverUrl: 'Files/cover.jpg',
      pageCoverUrl: 'Files/page.jpg',
      bookFile: 'Files/dune.epub',
      readerPosition: 'epubcfi(/6/4!/4)',
      readPercent: 42,
    });
  });

  it('falls back to legacy cover_url and book_file', () => {
    expect(resolveBookFields({ cover_url: 'Files/c.jpg', book_file: 'Files/b.epub' })).toMatchObject({
      bookCoverUrl: 'Files/c.jpg',
      bookFile: 'Files/b.epub',
    });
  });

  it('parses space-separated book cover line', () => {
    const parsed = parseFrontmatter('---\nBook_cover Files/cover.jpg\n---\n');
    expect(parsed?.data[FM_BOOK_COVER]).toBe('Files/cover.jpg');
  });

  it('writes canonical keys and drops legacy aliases', () => {
    const doc = '---\ncover_url: Files/old.jpg\nauthor: A\n---\nBody';
    const next = setFrontmatterField(doc, FM_BOOK_COVER, 'Files/cover.jpg');
    expect(next).toContain('Book_cover: Files/cover.jpg');
    expect(next).not.toContain('cover_url');
  });

  it('formats read percent from fraction', () => {
    expect(formatReadPercent(0.426)).toBe('43');
    expect(parseReadPercent('43%')).toBe(43);
  });
});
