import { useCallback, useEffect, useMemo, useRef, useState, type Dispatch } from 'react';
import { existingFiles } from '../modules/documents/fileGateway';
import {
  WorkspaceSession,
  stateFailure,
  type StateFailure,
  type GraphCameraState,
  type SessionTab,
  type TabsAction,
  type TabsState,
  type ViewState,
} from '../modules/ui-state';
import { loadTabSessionCache, saveTabSessionCache } from '../modules/workspace/uiPersist';
import { comparablePath } from '../modules/paths';
import { createEmptySessionTab } from './tabWorkspace';

interface UseTabSessionInput {
  workspacePath: string | null;
  workspaceReady: boolean;
  state: TabsState;
  dispatch: Dispatch<TabsAction>;
}

export function useTabSession({
  workspacePath,
  workspaceReady,
  state,
  dispatch,
}: UseTabSessionInput) {
  const [revision, setRevision] = useState(0);
  const [viewRevision, setViewRevision] = useState(0);
  const [sessionReady, setSessionReady] = useState(false);
  const sessionRef = useRef<WorkspaceSession | null>(null);
  const touchedRef = useRef(false);
  const generationRef = useRef(0);
  const prevWorkspaceRef = useRef<string | null | undefined>(undefined);
  const resolvingRef = useRef(new Set<string>());
  const [stateError, setStateError] = useState<StateFailure | null>(null);
  const degrade = useCallback((error: unknown, sessionFailed = false) => {
    setStateError((current) => current ?? stateFailure(error, sessionFailed));
  }, []);

  useEffect(() => {
    const prev = prevWorkspaceRef.current;
    prevWorkspaceRef.current = workspacePath;
    if (prev === undefined || prev === workspacePath) return;
    generationRef.current += 1;
    sessionRef.current?.dispose();
    sessionRef.current = null;
    resolvingRef.current.clear();
    setStateError(null);
    touchedRef.current = false;
    setSessionReady(false);
    const cached = workspacePath ? loadTabSessionCache(workspacePath) : null;
    if (cached) {
      dispatch({ type: 'restore', tabs: cached.tabs, activeTabId: cached.activeTabId });
      return;
    }
    dispatch({ type: 'reset', tab: createEmptySessionTab() });
  }, [dispatch, workspacePath]);

  useEffect(() => {
    if (!workspacePath || !workspaceReady) return;
    const generation = ++generationRef.current;
    let opened: WorkspaceSession | null = null;
    let cancelled = false;
    void WorkspaceSession.open(workspacePath).then(async (session) => {
      opened = session;
      const loaded = session.restoredTabs();
      const documentPaths = loaded
        .filter((tab) => tab.kind === 'document')
        .map((tab) => tab.path);
      const available = new Set((await existingFiles(documentPaths)).map(comparablePath));
      if (cancelled || generation !== generationRef.current) {
        session.dispose();
        return;
      }
      sessionRef.current = session;
      const restored = WorkspaceSession.restorable(
        loaded,
        (path) => available.has(comparablePath(path)),
      );
      for (const tab of loaded) {
        if (isMissingDocument(tab, available)) {
          void session.markDocumentMissing(tab.documentId!)
            .catch((error) => console.error('Failed to mark missing document', error));
        }
      }
      if (!touchedRef.current && restored.length > 0) {
        const activeTabId = restored.some((tab) => tab.tabId === session.loaded.activeTabId)
          ? session.loaded.activeTabId!
          : restored[0].tabId;
        dispatch({ type: 'restore', tabs: restored, activeTabId });
      }
      setRevision((value) => value + 1);
      setSessionReady(true);
    }).catch((error) => {
      opened?.dispose();
      console.error('Failed to restore workspace session', error);
      if (!cancelled && generation === generationRef.current) {
        degrade(error, true);
        setSessionReady(true);
      }
    });
    return () => {
      cancelled = true;
      opened?.dispose();
    };
  }, [dispatch, workspacePath, workspaceReady]);

  useEffect(() => {
    const session = sessionRef.current;
    if (!session) return;
    for (const tab of state.tabs) {
      const identityPath = tab.identityPath ?? tab.path;
      if (tab.kind !== 'document' || tab.documentId) continue;
      const key = `${tab.tabId}\0${identityPath}\0${tab.path}`;
      if (resolvingRef.current.has(key)) continue;
      resolvingRef.current.add(key);
      void session.resolveDocument(identityPath, tab.path).then((documentId) => {
        if (sessionRef.current === session) {
          dispatch({ type: 'resolve-identity', tabId: tab.tabId, path: tab.path, documentId });
        }
      }).catch((error) => {
        console.error('Failed to resolve document identity', error);
        if (sessionRef.current === session) degrade(error);
      }).finally(() => resolvingRef.current.delete(key));
    }
  }, [degrade, dispatch, revision, state.tabs, stateError]);

  const unresolved = useMemo(
    () => state.tabs.some((tab) => tab.kind === 'document' && !tab.documentId),
    [state.tabs],
  );

  const activeDocumentId = useMemo(
    () => state.tabs.find((tab) => tab.tabId === state.activeTabId)?.documentId ?? null,
    [state.activeTabId, state.tabs],
  );

  useEffect(() => {
    const session = sessionRef.current;
    if (!session || !activeDocumentId || stateError !== null) return;
    let cancelled = false;
    void session.ensureViewLoaded(activeDocumentId).then(() => {
      if (!cancelled && sessionRef.current === session) {
        setViewRevision((value) => value + 1);
      }
    }).catch((error) => {
      console.error('Failed to load document view', error);
      if (!cancelled && sessionRef.current === session) degrade(error);
    });
    return () => {
      cancelled = true;
    };
  }, [activeDocumentId, degrade, revision, stateError]);

  useEffect(() => {
    if (unresolved) return;
    sessionRef.current?.queueTabsSnapshot(state.tabs, state.activeTabId);
  }, [revision, state.tabs, unresolved]);

  useEffect(() => {
    if (unresolved) return;
    sessionRef.current?.queueActiveTab(state.activeTabId);
  }, [revision, state.activeTabId, unresolved]);

  useEffect(() => {
    if (unresolved || !workspacePath || !sessionReady) return;
    saveTabSessionCache(workspacePath, state.tabs, state.activeTabId);
  }, [revision, sessionReady, state.activeTabId, state.tabs, unresolved, workspacePath]);

  const touch = useCallback(() => {
    touchedRef.current = true;
  }, []);

  const trackRename = useCallback((tab: SessionTab | undefined, newPath: string) => {
    if (!tab?.documentId) return;
    void sessionRef.current?.renameDocument(tab.documentId, newPath)
      .catch((error) => console.error('Failed to track document rename', error));
  }, []);

  const loadedView = useCallback((documentId: string) => {
    return sessionRef.current?.loadedView(documentId) ?? null;
  }, []);

  const isViewLoaded = useCallback((documentId: string) => {
    return sessionRef.current?.isViewLoaded(documentId) ?? false;
  }, []);

  const queueView = useCallback((view: ViewState) => {
    sessionRef.current?.queueView(view);
  }, []);

  const queueGraphCamera = useCallback((camera: GraphCameraState) => {
    sessionRef.current?.queueGraphCamera(camera);
  }, []);

  const readGraphCamera = useCallback(() => sessionRef.current?.graphCamera() ?? null, []);

  return {
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
  };
}

function isMissingDocument(tab: SessionTab, available: Set<string>): boolean {
  return tab.kind === 'document'
    && !!tab.documentId
    && !available.has(comparablePath(tab.path));
}
