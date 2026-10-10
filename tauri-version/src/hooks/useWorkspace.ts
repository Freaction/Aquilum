import { useCallback, useEffect, useRef, useState } from 'react';
import { readDirectory, type WorkspaceItem } from '../modules/documents/fileGateway';
import { loadExpandedFolderPaths } from '../modules/workspace/uiPersist';
import {
  lastOpenedWorkspace,
  setWorkspaceHomePage,
  workspaceHomePage,
} from '../modules/workspaces';
import { comparablePath, fileName, parentDirectory } from '../modules/paths';
import { t } from '../i18n';
import { useWorkspaceEvents } from './useWorkspaceEvents';

export function useWorkspace() {
  const generationRef = useRef(0);
  const pendingRef = useRef(new Map<string, Promise<boolean>>());
  const [workspacePath, setWorkspacePath] = useState<string | null>(null);
  const [directories, setDirectories] = useState<Map<string, WorkspaceItem[]>>(new Map());
  const [loadingDirectories, setLoadingDirectories] = useState<Set<string>>(new Set());
  const [workspaceReady, setWorkspaceReady] = useState(false);
  const [homePage, setHomePageValue] = useState('');
  const [workspaceRestoring, setWorkspaceRestoring] = useState(true);
  const [openFailedPath, setOpenFailedPath] = useState<string | null>(null);
  const workspacePathRef = useRef(workspacePath);
  workspacePathRef.current = workspacePath;

  const fetchDirectory = useCallback((path: string): Promise<boolean> => {
    const pending = pendingRef.current.get(path);
    if (pending) return pending;
    const generation = generationRef.current;
    setLoadingDirectories((current) => new Set(current).add(path));
    const request = readDirectory(path).then((items) => {
      if (generation !== generationRef.current) return false;
      setDirectories((current) => new Map(current).set(path, items));
      return true;
    }).catch((error) => {
      if (generation === generationRef.current) {
        console.error('Failed to read directory', error);
      }
      return false;
    }).finally(() => {
      const isCurrent = pendingRef.current.get(path) === request;
      if (isCurrent) pendingRef.current.delete(path);
      if (isCurrent && generation === generationRef.current) {
        setLoadingDirectories((current) => {
          const next = new Set(current);
          next.delete(path);
          return next;
        });
      }
    });
    pendingRef.current.set(path, request);
    return request;
  }, []);

  const openWorkspace = useCallback(async (path: string) => {
    const generation = ++generationRef.current;
    pendingRef.current.clear();
    setOpenFailedPath(null);

    const expandedPaths = loadExpandedFolderPaths(path);
    setLoadingDirectories(new Set([path, ...expandedPaths]));

    try {
      const [rootItems, home, ...expandedEntries] = await Promise.all([
        readDirectory(path),
        workspaceHomePage(path),
        ...expandedPaths.map(async (folderPath) => {
          try {
            return [folderPath, await readDirectory(folderPath)] as const;
          } catch (error) {
            console.error('Failed to read expanded directory', error);
            return null;
          }
        }),
      ]);
      if (generation !== generationRef.current) return false;

      const next = new Map<string, WorkspaceItem[]>([[path, rootItems]]);
      for (const entry of expandedEntries) {
        if (entry) next.set(entry[0], entry[1]);
      }

      setDirectories(next);
      setHomePageValue(home);
      setWorkspacePath(path);
      setWorkspaceReady(true);
      return true;
    } catch (error) {
      if (generation === generationRef.current) {
        console.error('Failed to open workspace', error);
        setOpenFailedPath(path);
      }
      return false;
    } finally {
      if (generation === generationRef.current) {
        setLoadingDirectories(new Set());
      }
    }
  }, []);

  const refreshDirectory = useCallback(async (path: string): Promise<boolean> => {
    await pendingRef.current.get(path);
    return fetchDirectory(path);
  }, [fetchDirectory]);

  useEffect(() => {
    let cancelled = false;
    void lastOpenedWorkspace()
      .then((path) => (path && !cancelled ? openWorkspace(path) : false))
      .catch((error) => {
        console.error('Failed to restore last workspace', error);
        return false;
      })
      .finally(() => {
        if (!cancelled) setWorkspaceRestoring(false);
      });
    return () => {
      cancelled = true;
      ++generationRef.current;
      pendingRef.current.clear();
    };
  }, [openWorkspace]);

  const changeHomePage = useCallback((value: string) => {
    const path = workspacePathRef.current;
    if (!path) return;
    setHomePageValue(value);
    void setWorkspaceHomePage(path, value)
      .catch((error) => console.error('Failed to save the workspace home page', error));
  }, []);

  const pruneDirectories = useCallback(() => {
    setDirectories((current) => pruneDirectoryCache(current, workspacePathRef.current));
  }, []);

  const patchFileInTree = useCallback((oldPath: string, patch: { id?: string; name?: string }) => {
    setDirectories((current) => patchFileEntry(current, oldPath, patch));
  }, []);

  useWorkspaceEvents({
    workspacePath,
    directories,
    refreshDirectory,
    pruneDirectories,
  });

  return {
    directories,
    files: workspacePath ? directories.get(workspacePath) ?? [] : [],
    loadingDirectories,
    workspacePath,
    workspaceName: workspacePath ? fileName(workspacePath) : t('workspace.none'),
    workspaceReady,
    workspaceRestoring,
    openFailedPath,
    homePage,
    changeHomePage,
    loadDirectory: fetchDirectory,
    openWorkspace,
    patchFileInTree,
  };
}

function patchFileEntry(
  directories: Map<string, WorkspaceItem[]>,
  oldPath: string,
  patch: { id?: string; name?: string },
): Map<string, WorkspaceItem[]> {
  const parent = parentDirectory(oldPath);
  const parentKey = [...directories.keys()].find(
    (key) => comparablePath(key) === comparablePath(parent),
  );
  if (!parentKey) return directories;
  const items = directories.get(parentKey);
  if (!items) return directories;

  const target = comparablePath(oldPath);
  let changed = false;
  const nextItems = items.map((item) => {
    if (comparablePath(item.id) !== target) return item;
    changed = true;
    return {
      ...item,
      id: patch.id ?? item.id,
      name: patch.name ?? item.name,
    };
  });
  if (!changed) return directories;
  return new Map(directories).set(parentKey, nextItems);
}

function pruneDirectoryCache(
  directories: Map<string, WorkspaceItem[]>,
  workspacePath: string | null,
): Map<string, WorkspaceItem[]> {
  if (!workspacePath || !directories.has(workspacePath)) return directories;
  const reachable = new Set<string>();
  const pending = [workspacePath];
  while (pending.length > 0) {
    const path = pending.pop()!;
    if (reachable.has(path)) continue;
    reachable.add(path);
    for (const item of directories.get(path) ?? []) {
      if (item.type === 'folder' && directories.has(item.id)) pending.push(item.id);
    }
  }
  if (reachable.size === directories.size) return directories;
  return new Map([...directories].filter(([path]) => reachable.has(path)));
}
