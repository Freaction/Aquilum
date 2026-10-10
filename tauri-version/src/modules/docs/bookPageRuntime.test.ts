import { describe, expect, it, vi } from 'vitest';
import {
  getPages,
  hydrateFromFm,
  publishBookPageProgress,
  subscribeBookPageRuntime,
} from './bookPageRuntime';

describe('bookPageRuntime', () => {
  it('hydrates and reads pages from FM', () => {
    hydrateFromFm('/books/hydrate.md', {
      author: 'Author',
      pages: '3/10',
      cover_url: 'Files/cover.jpg',
      book_file: 'Files/book.epub',
    });
    expect(getPages('/books/hydrate.md')).toEqual({ current: 3, total: 10 });
  });

  it('notifies subscribers of their own page progress only', () => {
    const listener = vi.fn();
    const unsubscribe = subscribeBookPageRuntime('/books/notify-a.md', listener);
    publishBookPageProgress('/books/notify-a.md', { current: 3, total: 10 });
    publishBookPageProgress('/books/notify-b.md', { current: 1, total: 5 });

    expect(listener).toHaveBeenCalledTimes(1);
    expect(listener).toHaveBeenCalledWith(expect.objectContaining({ pages: { current: 3, total: 10 } }));

    unsubscribe();
    publishBookPageProgress('/books/notify-a.md', { current: 4, total: 10 });
    expect(listener).toHaveBeenCalledTimes(1);
  });

  it('hydrateFromFm keeps live pages but loads book_file from FM', () => {
    publishBookPageProgress('/books/live.md', { current: 5, total: 10 });
    const entry = hydrateFromFm('/books/live.md', {
      author: 'Author',
      pages: '1/10',
      book_file: 'Files/book.epub',
    });
    expect(entry.bookFile).toBe('Files/book.epub');
    expect(getPages('/books/live.md')).toEqual({ current: 5, total: 10 });
  });

  it('publishes changed metadata without retaining stale non-empty values', () => {
    const listener = vi.fn();
    hydrateFromFm('/books/Old.md', {
      author: 'Old author',
      cover_url: 'Files/old.jpg',
      book_file: 'Files/old.epub',
    });
    const unsubscribe = subscribeBookPageRuntime('/books/Old.md', listener);
    hydrateFromFm('/books/Old.md', {
      author: 'New author',
      cover_url: 'Files/new.jpg',
      book_file: 'Files/new.epub',
    });

    expect(listener).toHaveBeenLastCalledWith(expect.objectContaining({
      title: 'Old',
      author: 'New author',
      cover: 'Files/new.jpg',
      bookFile: 'Files/new.epub',
    }));
    unsubscribe();
  });

  it('matches subscribers across slash variants on Windows paths', () => {
    const listener = vi.fn();
    const unsubscribe = subscribeBookPageRuntime('D:\\books\\slashes.md', listener);
    publishBookPageProgress('D:/books/slashes.md', { current: 2, total: 8 });
    expect(listener).toHaveBeenCalledWith(expect.objectContaining({ pages: { current: 2, total: 8 } }));
    unsubscribe();
  });
});
