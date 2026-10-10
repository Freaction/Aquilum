import { useEffect, useRef, useState, type RefObject } from 'react';
import { loadGraphSnapshot, type GraphSnapshot } from '../../modules/graph';
import type { GraphDateRange, GraphRenderer } from './renderer';
import type { NoteLabels } from './noteLabels';
import { readGraphFailure } from './readGraphFailure';

export interface GraphCounts {
  nodeCount: number;
  edgeCount: number;
}

interface GraphSnapshotInput {
  rendererRef: RefObject<GraphRenderer | null>;
  generation: number;
  labels: NoteLabels;
  workspacePath: string | null;
  indexRevision: number;
  attempt: number;
  reload: number;
  activated: boolean;
  inactive: boolean;
  rendererError: string | null;
}

export function useGraphSnapshot({
  rendererRef,
  generation,
  labels,
  workspacePath,
  indexRevision,
  attempt,
  reload,
  activated,
  inactive,
  rendererError,
}: GraphSnapshotInput) {
  const [counts, setCounts] = useState<GraphCounts | null>(null);
  const [range, setRange] = useState<GraphDateRange>({ oldest: 0, newest: 1 });
  const [snapshotError, setSnapshotError] = useState<string | null>(null);
  const snapshotRef = useRef<GraphSnapshot | null>(null);
  const loadedKeyRef = useRef<string | null>(null);
  const loadedWorkspaceRef = useRef<string | null>(null);
  const shownRef = useRef(false);
  const refittedRef = useRef(0);

  useEffect(() => {
    if (!activated || inactive || !workspacePath || rendererError !== null) return;
    const snapshotKey = `${attempt} ${reload} ${workspacePath} ${indexRevision}`;
    if (loadedKeyRef.current === snapshotKey) return;
    loadedKeyRef.current = snapshotKey;
    const keepCamera = loadedWorkspaceRef.current === workspacePath;
    if (!keepCamera) shownRef.current = false;
    loadedWorkspaceRef.current = workspacePath;
    const refits = reload !== refittedRef.current;
    refittedRef.current = reload;
    let cancelled = false;
    setSnapshotError(null);
    loadGraphSnapshot(workspacePath)
      .then((snapshot: GraphSnapshot) => {
        if (cancelled) return;
        const renderer = rendererRef.current;
        snapshotRef.current = snapshot;
        labels.adopt(snapshot.epoch);
        renderer?.setSnapshot(snapshot, keepCamera);
        setRange(renderer?.createdRange() ?? { oldest: 0, newest: 1 });
        setCounts({ nodeCount: snapshot.nodeCount, edgeCount: snapshot.edgeCount });
        shownRef.current = true;
        if (refits) renderer?.fit();
      })
      .catch((cause) => {
        if (cancelled) return;
        loadedKeyRef.current = null;
        console.error('Failed to load the graph snapshot', cause);
        if (shownRef.current) return;
        setCounts(null);
        setSnapshotError(readGraphFailure(cause));
      });
    return () => { cancelled = true; };
  }, [
    activated,
    attempt,
    inactive,
    indexRevision,
    labels,
    reload,
    rendererError,
    rendererRef,
    workspacePath,
  ]);

  useEffect(() => {
    const renderer = rendererRef.current;
    const snapshot = snapshotRef.current;
    if (!renderer || !snapshot || renderer.hasSnapshot()) return;
    renderer.setSnapshot(snapshot);
  }, [generation, rendererRef]);

  return { counts, range, snapshotError };
}
