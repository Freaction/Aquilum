import { useCallback, useEffect, useRef, useState } from 'react';
import {
  canNavigateBack,
  canNavigateForward,
  emptyNavigationHistory,
  pushNavigationEntry,
  stepNavigationHistory,
  type NavigationHistoryState,
} from '../modules/navigationHistory';
import {
  loadNavigationHistory,
  saveNavigationHistory,
} from '../modules/workspace/uiPersist';

export function useNavigationHistory(workspacePath: string | null) {
  const historyRef = useRef<NavigationHistoryState>(emptyNavigationHistory);
  const workspaceRef = useRef(workspacePath);
  const [flags, setFlags] = useState({ back: false, forward: false });

  const commit = useCallback((state: NavigationHistoryState) => {
    historyRef.current = state;
    const back = canNavigateBack(state);
    const forward = canNavigateForward(state);
    setFlags((prev) => (
      prev.back === back && prev.forward === forward ? prev : { back, forward }
    ));
    if (workspaceRef.current) saveNavigationHistory(workspaceRef.current, state);
  }, []);

  useEffect(() => {
    workspaceRef.current = workspacePath;
    const next = workspacePath
      ? loadNavigationHistory(workspacePath)
      : emptyNavigationHistory;
    historyRef.current = next;
    setFlags({
      back: canNavigateBack(next),
      forward: canNavigateForward(next),
    });
  }, [workspacePath]);

  const push = useCallback((path: string) => {
    commit(pushNavigationEntry(historyRef.current, path));
  }, [commit]);

  const go = useCallback((delta: -1 | 1): string | null => {
    const next = stepNavigationHistory(historyRef.current, delta);
    if (!next.path) return null;
    commit(next.state);
    return next.path;
  }, [commit]);

  return {
    push,
    go,
    canGoBack: flags.back,
    canGoForward: flags.forward,
  };
}
