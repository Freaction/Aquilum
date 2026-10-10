import { useEffect, useRef } from 'react';
import type { WorkspaceItem } from '../modules/documents/fileGateway';
import { comparablePath, parentDirectory } from '../modules/paths';
import { forgetAttachmentUrls } from '../modules/docs/vaultAttachments';
import { useTauriEvent } from './useTauriEvent';

const EVENT_NAME = 'workspace-changed';
const EVENT_DELAY_MS = 60;
const REFRESH_CONCURRENCY = 4;

interface WorkspaceEventsOptions {
  workspacePath: string | null;
  directories: ReadonlyMap<string, WorkspaceItem[]>;
  refreshDirectory: (path: string) => Promise<boolean>;
  pruneDirectories: () => void;
}

export function useWorkspaceEvents(options: WorkspaceEventsOptions): void {
  const optionsRef = useRef(options);
  const changedPathsRef = useRef(new Set<string>());
  const timerRef = useRef<number | null>(null);
  const mountedRef = useRef(false);
  optionsRef.current = options;

  useEffect(() => {
    mountedRef.current = true;
    const changedPaths = changedPathsRef.current;
    return () => {
      mountedRef.current = false;
      if (timerRef.current !== null) window.clearTimeout(timerRef.current);
      changedPaths.clear();
    };
  }, []);

  useTauriEvent<string[]>(EVENT_NAME, (paths) => {
    forgetAttachmentUrls();
    paths.forEach((path) => changedPathsRef.current.add(path));
    if (timerRef.current !== null) window.clearTimeout(timerRef.current);
    timerRef.current = window.setTimeout(() => {
      timerRef.current = null;
      void refreshChanged(changedPathsRef.current, optionsRef.current, mountedRef);
    }, EVENT_DELAY_MS);
  });
}

async function refreshChanged(
  changedPaths: Set<string>,
  options: WorkspaceEventsOptions,
  mounted: { current: boolean },
): Promise<void> {
  const changed = [...changedPaths];
  changedPaths.clear();
  const targets = affectedDirectories(options.workspacePath, options.directories, changed);
  await runBounded(targets, options.refreshDirectory);
  if (mounted.current) options.pruneDirectories();
}

export function affectedDirectories(
  workspacePath: string | null,
  directories: ReadonlyMap<string, WorkspaceItem[]>,
  changedPaths: string[],
): string[] {
  if (!workspacePath) return [];
  const loaded = new Map([...directories.keys()].map((path) => [comparablePath(path), path]));
  const root = comparablePath(workspacePath);
  const targets = new Set<string>();
  for (const changed of changedPaths) {
    const normalized = comparablePath(changed);
    if (normalized === root) {
      const target = loaded.get(root);
      if (target) targets.add(target);
      continue;
    }
    const parent = parentDirectory(normalized);
    const target = loaded.get(parent) ?? ancestorMissingTheBranch(directories, loaded, root, parent);
    if (target) targets.add(target);
  }
  return [...targets];
}

function ancestorMissingTheBranch(
  directories: ReadonlyMap<string, WorkspaceItem[]>,
  loaded: ReadonlyMap<string, string>,
  root: string,
  unloadedParent: string,
): string | undefined {
  let branch = unloadedParent;
  let directory = parentDirectory(branch);
  while (directory.length >= root.length) {
    const ancestor = loaded.get(directory);
    if (ancestor) {
      const items = directories.get(ancestor) ?? [];
      const alreadyListed = items.some((item) => comparablePath(item.id) === branch);
      return alreadyListed ? undefined : ancestor;
    }
    branch = directory;
    directory = parentDirectory(directory);
  }
  return undefined;
}

async function runBounded(
  paths: string[],
  operation: (path: string) => Promise<boolean>,
): Promise<void> {
  let cursor = 0;
  const worker = async () => {
    while (cursor < paths.length) {
      const path = paths[cursor];
      cursor += 1;
      await operation(path);
    }
  };
  await Promise.all(Array.from({ length: Math.min(paths.length, REFRESH_CONCURRENCY) }, worker));
}

