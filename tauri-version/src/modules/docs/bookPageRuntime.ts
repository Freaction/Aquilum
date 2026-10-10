import type { Pages } from './bookProgress';
import { parsePages } from './bookProgress';
import { comparablePath, fileStem } from '../paths';
import { type BookMetadata, resolveBookFields } from './frontmatter';

export type BookPageRuntimeEntry = {
  title: string;
  author: string;
  pages: Pages | null;
  cover: string;
  bookFile: string;
  pagesGeneration: number;
};

type RuntimeListener = (entry: BookPageRuntimeEntry) => void;

const MAX_RUNTIME_ENTRIES = 64;

const byPath = new Map<string, BookPageRuntimeEntry>();
const listeners = new Map<string, Set<RuntimeListener>>();
const lru: string[] = [];

function touch(key: string): void {
  const index = lru.indexOf(key);
  if (index >= 0) lru.splice(index, 1);
  lru.push(key);
  evictIfNeeded();
}

function evictIfNeeded(): void {
  let scans = 0;
  while (lru.length > MAX_RUNTIME_ENTRIES && scans < lru.length) {
    const key = lru.shift()!;
    if (listeners.get(key)?.size) {
      lru.push(key);
      scans += 1;
      continue;
    }
    byPath.delete(key);
    scans = 0;
  }
}

function notify(key: string, entry: BookPageRuntimeEntry): void {
  const set = listeners.get(key);
  if (!set) return;
  for (const listener of set) listener(entry);
}

function emptyEntry(pages: Pages | null = null): BookPageRuntimeEntry {
  return { title: '', author: '', pages, cover: '', bookFile: '', pagesGeneration: 0 };
}

export function getPages(pagePath: string): Pages | null {
  return byPath.get(comparablePath(pagePath))?.pages ?? null;
}

export function publishBookPageProgress(pagePath: string, pages: Pages | null): void {
  const key = comparablePath(pagePath);
  const entry = byPath.get(key) ?? emptyEntry(pages);
  entry.pages = pages;
  entry.pagesGeneration += 1;
  byPath.set(key, entry);
  touch(key);
  notify(key, entry);
}

export function hydrateFromFm(
  pagePath: string,
  fm: BookMetadata | null | undefined,
): BookPageRuntimeEntry {
  const key = comparablePath(pagePath);
  const prev = byPath.get(key);
  const fmPages = typeof fm?.pages === 'string' ? parsePages(fm.pages) : null;
  const fields = resolveBookFields(fm);
  const entry: BookPageRuntimeEntry = {
    title: fileStem(pagePath),
    author: typeof fm?.author === 'string' ? fm.author : '',
    cover: fields.bookCoverUrl ?? '',
    bookFile: fields.bookFile ?? '',
    pages: prev && prev.pagesGeneration > 0 ? prev.pages : fmPages,
    pagesGeneration: prev?.pagesGeneration ?? 0,
  };
  byPath.set(key, entry);
  touch(key);
  notify(key, entry);
  return entry;
}

export function subscribeBookPageRuntime(
  pagePath: string,
  listener: RuntimeListener,
): () => void {
  const key = comparablePath(pagePath);
  let set = listeners.get(key);
  if (!set) {
    set = new Set();
    listeners.set(key, set);
  }
  set.add(listener);
  touch(key);
  const cached = byPath.get(key);
  if (cached) listener(cached);
  return () => {
    set!.delete(listener);
    if (set!.size === 0) listeners.delete(key);
  };
}
