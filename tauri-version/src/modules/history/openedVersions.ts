import { useCallback, useSyncExternalStore } from 'react';
import type { NoteVersion } from './index';

export interface OpenedVersion {
  path: string;
  version: NoteVersion;
  scrollTop: number;
}

const opened = new Map<string, OpenedVersion>();
const listeners = new Set<() => void>();

function notify(): void {
  for (const listener of listeners) listener();
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

export function openVersion(tabId: string, path: string, version: NoteVersion): void {
  opened.set(tabId, { path, version, scrollTop: 0 });
  notify();
}

export function closeVersion(tabId: string, version?: NoteVersion): void {
  const current = opened.get(tabId);
  if (!current || (version && current.version.id !== version.id)) return;
  opened.delete(tabId);
  notify();
}

export function rememberScroll(tabId: string, scrollTop: number): void {
  const current = opened.get(tabId);
  if (current) current.scrollTop = scrollTop;
}

export function keepVersionsOf(tabIds: Iterable<string>): void {
  const alive = new Set(tabIds);
  let changed = false;
  for (const tabId of opened.keys()) {
    if (alive.has(tabId)) continue;
    opened.delete(tabId);
    changed = true;
  }
  if (changed) notify();
}

export function useOpenedVersion(tabId: string | null): OpenedVersion | null {
  const snapshot = useCallback(() => (tabId ? opened.get(tabId) ?? null : null), [tabId]);
  return useSyncExternalStore(subscribe, snapshot);
}
