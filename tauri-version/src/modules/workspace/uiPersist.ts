import { useCallback, useEffect, useRef, useState } from 'react';
import {
  emptyNavigationHistory,
  type NavigationHistoryState,
} from '../navigationHistory';
import { absolutePath, comparablePath, rebasedPath, relativePath } from '../paths';
import type { SessionTab } from '../ui-state';
import { clamp } from '../math';

function readJson<T>(key: string, fallback: T): T {
  try {
    const raw = localStorage.getItem(key);
    if (raw === null) return fallback;
    return JSON.parse(raw) as T;
  } catch {
    return fallback;
  }
}

function writeJson(key: string, value: unknown): void {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch {}
}

export function useLocalState<T>(key: string, fallback: T) {
  const keyRef = useRef(key);
  const [value, setValue] = useState(() => readJson(key, fallback));

  useEffect(() => {
    if (keyRef.current === key) return;
    keyRef.current = key;
    setValue(readJson(key, fallback));
  }, [fallback, key]);

  const set = useCallback((updater: T | ((current: T) => T)) => {
    setValue((current) => {
      const next = typeof updater === 'function'
        ? (updater as (current: T) => T)(current)
        : updater;
      writeJson(keyRef.current, next);
      return next;
    });
  }, []);

  return [value, set] as const;
}

function workspaceStorageKey(prefix: string, workspacePath: string): string {
  return `${prefix}:${comparablePath(workspacePath)}`;
}

function expandedKey(workspacePath: string): string {
  return workspaceStorageKey('aquilum_expanded_folders', workspacePath);
}

export function loadExpandedFolderPaths(workspacePath: string): string[] {
  return readJson<string[]>(expandedKey(workspacePath), []).flatMap((value) => (
    typeof value === 'string' && value.trim() ? [absolutePath(workspacePath, value)] : []
  ));
}

function saveExpandedFolderPaths(workspacePath: string, folderPaths: Iterable<string>): void {
  writeJson(
    expandedKey(workspacePath),
    [...folderPaths].flatMap((path) => {
      try {
        return [relativePath(workspacePath, path)];
      } catch {
        return [];
      }
    }),
  );
}

export function useExpandedFolders(workspacePath: string | null) {
  const pathRef = useRef(workspacePath);
  const [expandedFolders, setExpandedFoldersState] = useState(
    () => (workspacePath ? new Set(loadExpandedFolderPaths(workspacePath)) : new Set<string>()),
  );

  useEffect(() => {
    if (pathRef.current === workspacePath) return;
    pathRef.current = workspacePath;
    setExpandedFoldersState(
      workspacePath ? new Set(loadExpandedFolderPaths(workspacePath)) : new Set(),
    );
  }, [workspacePath]);

  const setExpandedFolders = useCallback((
    updater: Set<string> | ((current: Set<string>) => Set<string>),
  ) => {
    setExpandedFoldersState((current) => {
      const next = typeof updater === 'function' ? updater(current) : updater;
      if (pathRef.current) saveExpandedFolderPaths(pathRef.current, next);
      return next;
    });
  }, []);

  const followFolder = useCallback((from: string, to: string) => {
    setExpandedFolders((current) => (
      [...current].some((path) => rebasedPath(path, from, to))
        ? new Set([...current].map((path) => rebasedPath(path, from, to) ?? path))
        : current
    ));
  }, [setExpandedFolders]);

  return { expandedFolders, setExpandedFolders, followFolder };
}

function tabsCacheKey(workspacePath: string): string {
  return workspaceStorageKey('aquilum_tabs_cache', workspacePath);
}

interface StoredTabSession {
  tabs: SessionTab[];
  activeTabId: string;
}

export function loadTabSessionCache(workspacePath: string): StoredTabSession | null {
  const stored = readJson<StoredTabSession | null>(tabsCacheKey(workspacePath), null);
  if (!stored || !Array.isArray(stored.tabs) || stored.tabs.length === 0) return null;
  if (typeof stored.activeTabId !== 'string') return null;
  return stored;
}

export function saveTabSessionCache(
  workspacePath: string,
  tabs: StoredTabSession['tabs'],
  activeTabId: string,
): void {
  writeJson(tabsCacheKey(workspacePath), { tabs, activeTabId } satisfies StoredTabSession);
}

function navigationKey(workspacePath: string): string {
  return workspaceStorageKey('aquilum_nav_history', workspacePath);
}

interface StoredNavigationHistory {
  entries: string[];
  index: number;
}

export function loadNavigationHistory(workspacePath: string): NavigationHistoryState {
  const stored = readJson<StoredNavigationHistory | null>(navigationKey(workspacePath), null);
  if (!stored || !Array.isArray(stored.entries)) return emptyNavigationHistory;

  const entries = stored.entries.flatMap((value) => (
    typeof value === 'string' && value.trim() ? [absolutePath(workspacePath, value)] : []
  ));
  if (entries.length === 0) return emptyNavigationHistory;

  const rawIndex = typeof stored.index === 'number' ? stored.index : 0;
  const index = clamp(rawIndex, 0, entries.length - 1);
  return { entries, index };
}

export function saveNavigationHistory(
  workspacePath: string,
  state: NavigationHistoryState,
): void {
  const entries: string[] = [];
  let index = -1;
  for (let i = 0; i < state.entries.length; i += 1) {
    try {
      entries.push(relativePath(workspacePath, state.entries[i]!));
      if (i === state.index) index = entries.length - 1;
    } catch {}
  }
  writeJson(navigationKey(workspacePath), { entries, index } satisfies StoredNavigationHistory);
}

// режим просмотра скрытых элементов сохраняется для каждого хранилища.
export function useExplorerShowHidden(workspacePath: string | null) {
  return useLocalState(workspaceStorageKey('aquilum_explorer_show_hidden', workspacePath ?? ''), false);
}
