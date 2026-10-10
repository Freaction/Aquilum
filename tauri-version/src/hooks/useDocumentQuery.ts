import { useEffect, useState } from 'react';
import { useStableCallback } from './useStableCallback';

interface DocumentQuery<T> {
  scope: string;
  workspacePath: string | null;
  documentPath: string | null;
  indexReady: boolean;
  indexRevision: number;
  enabled: boolean;
  load: (workspacePath: string, documentPath: string) => Promise<T>;
}

interface QueryState<T> {
  cacheKey: string;
  result: T | null;
  failed: boolean;
}

export function useDocumentQuery<T>({
  scope,
  workspacePath,
  documentPath,
  indexReady,
  indexRevision,
  enabled,
  load,
}: DocumentQuery<T>) {
  const [state, setState] = useState<QueryState<T> | null>(null);
  const run = useStableCallback(load);
  const canLoad = enabled && indexReady && workspacePath && documentPath;
  const cacheKey = canLoad ? `${scope}\0${workspacePath}\0${documentPath}` : null;
  const requestKey = canLoad ? `${cacheKey}\0${indexRevision}` : null;

  useEffect(() => {
    if (!requestKey || !cacheKey || !workspacePath || !documentPath) return;
    let current = true;
    void run(workspacePath, documentPath)
      .then((result) => {
        if (current) setState({ cacheKey, result, failed: false });
      })
      .catch((error) => {
        if (!current) return;
        console.error(`Failed to load ${scope}`, error);
        setState({ cacheKey, result: null, failed: true });
      });
    return () => {
      current = false;
    };
  }, [cacheKey, documentPath, requestKey, run, scope, workspacePath]);

  const settled = state?.cacheKey === cacheKey ? state : null;
  return {
    result: settled?.result ?? null,
    failed: settled?.failed ?? false,
    loading: Boolean(requestKey && !settled),
  };
}
