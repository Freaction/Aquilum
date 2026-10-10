import {
  loadReaderState as loadReaderStateRemote,
  resolveWorkspace,
  saveReaderState as saveReaderStateRemote,
} from '../ui-state/gateway';

import type { Pages } from './bookProgress';

type ReaderStateEntry = {
  cfi?: string;
  current: number;
};

const FLUSH_MS = 400;
const RETRY_MS = 5_000;

const memory: Record<string, ReaderStateEntry> = {};
const dirty = new Set<string>();
let workspaceId: string | null = null;
let resolvedWorkspace: { path: string; id: Promise<string> } | null = null;
let flushTimer: ReturnType<typeof setTimeout> | null = null;

function key(bookFile: string): string {
  return `${workspaceId ?? ''}\n${bookFile}`;
}

function scheduleFlush(delayMs: number): void {
  if (flushTimer) return;
  flushTimer = setTimeout(() => {
    flushTimer = null;
    void flushReaderState();
  }, delayMs);
}

function workspaceIdFor(workspacePath: string): Promise<string> {
  if (resolvedWorkspace?.path !== workspacePath) {
    const id = resolveWorkspace(workspacePath, Date.now());
    resolvedWorkspace = { path: workspacePath, id };
    id.catch(() => {
      if (resolvedWorkspace?.id === id) resolvedWorkspace = null;
    });
  }
  return resolvedWorkspace.id;
}

async function ensureWorkspace(workspacePath: string | null | undefined): Promise<string | null> {
  if (!workspacePath) {
    workspaceId = null;
    return null;
  }
  try {
    workspaceId = await workspaceIdFor(workspacePath);
    return workspaceId;
  } catch (error) {
    console.error('Failed to resolve the workspace for reader state', error);
    workspaceId = null;
    return null;
  }
}

export async function flushReaderState(): Promise<void> {
  if (flushTimer) {
    clearTimeout(flushTimer);
    flushTimer = null;
  }
  const ws = workspaceId;
  if (!ws) {
    dirty.clear();
    return;
  }
  const books = [...dirty];
  dirty.clear();
  for (const bookFile of books) {
    const entry = memory[key(bookFile)];
    if (!entry) continue;
    try {
      await saveReaderStateRemote({
        workspaceId: ws,
        bookFile,
        current: entry.current,
        cfi: entry.cfi ?? null,
        nowMs: Date.now(),
      });
    } catch (error) {
      console.error('Failed to save reader position', error);
      dirty.add(bookFile);
    }
  }
  if (dirty.size > 0) scheduleFlush(RETRY_MS);
}

export async function hydrateReaderState(
  workspacePath: string | null | undefined,
  bookFile: string,
): Promise<ReaderStateEntry | null> {
  const ws = await ensureWorkspace(workspacePath);
  const cached = memory[key(bookFile)];
  if (cached) return cached;
  if (!ws) return null;
  try {
    const loaded = await loadReaderStateRemote(ws, bookFile);
    if (!loaded) return null;
    const entry: ReaderStateEntry = {
      current: loaded.current,
      cfi: loaded.cfi ?? undefined,
    };
    memory[key(bookFile)] = entry;
    return entry;
  } catch (error) {
    console.error('Failed to load reader position', error);
    return null;
  }
}

function readerState(bookFile: string): ReaderStateEntry | null {
  return memory[key(bookFile)] ?? null;
}

export function setReaderState(bookFile: string, current: number, cfi?: string | null): void {
  const k = key(bookFile);
  const prev = memory[k];
  const nextCfi = cfi === null ? undefined : (cfi ?? prev?.cfi);
  if (prev?.current === current && prev.cfi === nextCfi) return;
  memory[k] = { current, cfi: nextCfi };
  dirty.add(bookFile);
  scheduleFlush(FLUSH_MS);
}

export function resolveReaderOpenTarget(
  bookFile: string,
  pages: Pages,
  initialCfi?: string,
): string | { fraction: number } | undefined {
  if (initialCfi) return initialCfi;
  if (pages.current <= 0) return undefined;
  const cache = readerState(bookFile);
  if (cache?.cfi && cache.current === pages.current) return cache.cfi;
  if (cache?.cfi && cache.current !== pages.current) {
    setReaderState(bookFile, pages.current, null);
  }
  return { fraction: pages.current / pages.total };
}
