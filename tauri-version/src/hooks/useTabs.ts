import { useCallback, useMemo, useReducer } from 'react';
import { GRAPH_TAB_PATH, tabsReducer, type TabsState } from '../modules/ui-state';
import {
  createEmptySessionTab,
  createGraphSessionTab,
  createLinkedFile as createLinkedFileOnDisk,
} from './tabWorkspace';
import { createUniqueFile } from '../modules/documents/documentFactory';
import { useTabSession } from './useTabSession';
import { isMarkdownPath } from '../modules/documents/fileGateway';
import type { LinkDisposition } from '../modules/links';

function createInitialState(): TabsState {
  const tab = createEmptySessionTab();
  return { tabs: [tab], activeTabId: tab.tabId };
}

export function useTabs(
  workspacePath: string | null,
  workspaceReady: boolean,
) {
  const [state, dispatch] = useReducer(
    tabsReducer,
    undefined,
    createInitialState,
  );
  const {
    touch,
    trackRename,
    loadedView,
    isViewLoaded,
    queueView,
    readGraphCamera,
    queueGraphCamera,
    sessionReady,
    viewRevision,
    stateError,
  } = useTabSession({
    workspacePath,
    workspaceReady,
    state,
    dispatch,
  });

  const pathsByTabId = useMemo(
    () => new Map(state.tabs.map((tab) => [tab.tabId, tab.path])),
    [state.tabs],
  );
  const activeFile = pathsByTabId.get(state.activeTabId) ?? null;
  const openFiles = useMemo(() => state.tabs.map((tab) => tab.path), [state.tabs]);

  const openGraph = useCallback(() => {
    touch();
    dispatch({ type: 'open-graph', tab: createGraphSessionTab() });
  }, [touch]);

  const activateFile = useCallback((
    path: string,
    options?: { disposition?: LinkDisposition },
  ) => {
    if (path === GRAPH_TAB_PATH) {
      openGraph();
      return;
    }
    const disposition = options?.disposition ?? 'current';
    if (disposition === 'current' && state.tabs.some((tab) => tab.path === path)) {
      touch();
      dispatch({ type: 'select', path });
      return;
    }
    if (!isMarkdownPath(path)) return;
    touch();
    dispatch(disposition === 'new-tab'
      ? { type: 'open-file-new-tab', path, tabId: crypto.randomUUID() }
      : { type: 'open-file', path, tabId: crypto.randomUUID() });
  }, [openGraph, state.tabs, touch]);

  const closeTab = useCallback((path: string) => {
    touch();
    dispatch({ type: 'close', path, fallback: createEmptySessionTab() });
  }, [touch]);

  const reorderTabs = useCallback((from: number, to: number) => {
    touch();
    dispatch({ type: 'reorder', from, to });
  }, [touch]);

  const newTab = useCallback(() => {
    touch();
    dispatch({ type: 'new-tab', tab: createEmptySessionTab() });
  }, [touch]);

  const createNewFile = useCallback(async (preferredTitle?: string, folder?: string) => {
    if (!workspacePath) return;
    const uniquePath = await createUniqueFile(folder ?? workspacePath, preferredTitle);
    if (!uniquePath) return;
    touch();
    dispatch({ type: 'open-file', path: uniquePath, tabId: crypto.randomUUID() });
  }, [touch, workspacePath]);

  const createFromTemplate = useCallback(async (content: string) => {
    if (!workspacePath) return;
    const path = await createUniqueFile(workspacePath, undefined, content);
    if (!path) return;
    touch(); dispatch({ type: 'open-file', path, tabId: crypto.randomUUID() });
  }, [touch, workspacePath]);

  const createLinkedFile = useCallback(async (target: string) => {
    if (!workspacePath) return null;
    return createLinkedFileOnDisk(workspacePath, target);
  }, [workspacePath]);

  const handleExternalRename = useCallback((oldPath: string, newPath: string) => {
    if (!state.tabs.some((tab) => tab.path === oldPath)) return;
    touch();
    trackRename(state.tabs.find((tab) => tab.path === oldPath), newPath);
    dispatch({ type: 'rename', oldPath, newPath });
  }, [state.tabs, touch, trackRename]);

  return {
    tabs: state.tabs,
    openFiles,
    activeFile,
    activeTabId: state.activeTabId,
    sessionReady,
    viewRevision,
    stateError,
    loadedView,
    isViewLoaded,
    onViewStateChange: queueView,
    readGraphCamera,
    onGraphCameraChange: queueGraphCamera,
    activateFile,
    closeTab,
    newTab,
    reorderTabs,
    openGraph,
    createNewFile,
    createFromTemplate,
    createLinkedFile,
    handleExternalRename,
  };
}
