import { useCallback, useEffect, useRef, useState } from 'react';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { whenIdle } from '../idle';
import { samePath } from '../paths';
import { getSearchIndexStatus, prepareSearchIndex, type SearchIndexStatus } from '../search';
import { LINKS_CHANGED_EVENT, type IndexRevisionEvent } from './events';

interface PendingPreparation {
  workspacePath: string;
  promise: Promise<void>;
}

function indexIdentity(status: SearchIndexStatus): string {
  return `${status.generation}:${status.revision}`;
}

export function useLinkIndex(workspacePath: string | null) {
  const pendingRef = useRef<PendingPreparation | null>(null);
  const seenRef = useRef<string | null>(null);
  const [ready, setReady] = useState(false);
  const [revision, setRevision] = useState(0);

  const ensureReady = useCallback((): Promise<void> => {
    if (!workspacePath) return Promise.resolve();
    const pending = pendingRef.current;
    if (pending?.workspacePath === workspacePath) return pending.promise;
    const next: PendingPreparation = {
      workspacePath,
      promise: prepareSearchIndex(workspacePath).then(() => undefined),
    };
    pendingRef.current = next;
    void next.promise.catch(() => {
      if (pendingRef.current === next) pendingRef.current = null;
    });
    return next.promise;
  }, [workspacePath]);

  useEffect(() => {
    let disposed = false;
    let unlisten: UnlistenFn | null = null;
    let cancelIdle: (() => void) | null = null;
    seenRef.current = null;
    setReady(false);
    setRevision((value) => value + 1);
    const adopt = (status: SearchIndexStatus) => {
      if (disposed) return;
      if (status.state !== 'ready') {
        setReady(false);
        return;
      }
      setReady(true);
      const identity = indexIdentity(status);
      if (seenRef.current === identity) return;
      seenRef.current = identity;
      setRevision((value) => value + 1);
    };
    const readStatus = () => {
      if (!workspacePath) return;
      void getSearchIndexStatus(workspacePath).then(adopt).catch((error) => {
        if (!disposed) console.error('Failed to read links index status', error);
      });
    };
    const prepare = () => {
      void ensureReady()
        .then(readStatus)
        .catch((error) => console.error('Failed to prepare links index', error));
    };
    void listen<IndexRevisionEvent>(LINKS_CHANGED_EVENT, (event) => {
      if (!samePath(event.payload.workspacePath, workspacePath)) return;
      readStatus();
    }).then((dispose) => {
      if (disposed) {
        dispose();
        return;
      }
      unlisten = dispose;
      cancelIdle = whenIdle(prepare);
    }).catch((error) => {
      console.error('Failed to listen for links updates', error);
      cancelIdle = whenIdle(prepare);
    });
    return () => {
      disposed = true;
      unlisten?.();
      cancelIdle?.();
    };
  }, [ensureReady]);

  return { ensureReady, ready, revision };
}
