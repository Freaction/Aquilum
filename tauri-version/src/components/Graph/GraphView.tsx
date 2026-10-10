import { plural, t } from '../../i18n';
import { useEffect, useRef, useState } from 'react';
import { GraphNotice, GraphStatus } from './GraphOverlay';
import { GraphSettings } from './GraphSettings';
import { MAX_EDGES_PER_FRAME } from './nodeMetrics';
import { NoteLabels } from './noteLabels';
import { loadGraphPaths } from '../../modules/graph';
import type { GraphCameraState } from '../../modules/ui-state';
import { useStableCallback } from '../../hooks/useStableCallback';
import { useGraphControls } from './useGraphControls';
import { useGraphRenderer } from './useGraphRenderer';
import { useGraphSnapshot } from './useGraphSnapshot';
import './GraphView.css';

interface GraphViewProps {
  workspacePath: string | null;
  indexRevision: number;
  inactive: boolean;
  sessionReady: boolean;
  readCamera: () => GraphCameraState | null;
  onCameraChange: (camera: GraphCameraState) => void;
  onOpenNote: (path: string) => void;
}

export function GraphView({
  workspacePath,
  indexRevision,
  inactive,
  sessionReady,
  readCamera,
  onCameraChange,
  onOpenNote,
}: GraphViewProps) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const cameraGenerationRef = useRef(0);
  const [attempt, setAttempt] = useState(0);
  const [reload, setReload] = useState(0);
  const [activated, setActivated] = useState(!inactive);

  useEffect(() => {
    if (!inactive) setActivated(true);
  }, [inactive]);

  const labelsRef = useRef<NoteLabels | null>(null);
  if (!labelsRef.current) {
    labelsRef.current = new NoteLabels(loadGraphPaths);
  }
  const labels = labelsRef.current;

  useEffect(() => {
    return () => {
      labels.detach();
    };
  }, [labels]);

  const visible = activated && !inactive;
  const generationRef = useRef(0);

  const handleCameraSettled = useStableCallback((camera: GraphCameraState) => {
    cameraGenerationRef.current = generationRef.current;
    onCameraChange(camera);
  });

  const { rendererRef, generation, rendererError, visibleNodes, attachScaleReadout } = useGraphRenderer({
    canvasRef,
    labels,
    visible,
    attempt,
    onOpenNote,
    onCameraSettled: handleCameraSettled,
  });

  generationRef.current = generation;

  const { counts, range, snapshotError } = useGraphSnapshot({
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
  });

  const { controls, createdShare, setCreatedShare, patchControls } = useGraphControls(
    rendererRef,
    generation,
    counts,
    range,
  );

  const readStoredCamera = useStableCallback(readCamera);

  useEffect(() => {
    cameraGenerationRef.current = 0;
  }, [workspacePath]);

  useEffect(() => {
    if (cameraGenerationRef.current === generation || !counts || !sessionReady) return;
    const stored = readStoredCamera();
    if (!stored) return;
    cameraGenerationRef.current = generation;
    rendererRef.current?.applyCamera(stored);
  }, [counts, generation, readStoredCamera, rendererRef, sessionReady]);

  const drawnEdges = counts ? Math.min(counts.edgeCount, MAX_EDGES_PER_FRAME) : 0;
  const status = !counts
    ? t('graph.loading')
    : visibleNodes < counts.nodeCount
      ? t('graph.notesVisible', { visible: visibleNodes, total: counts.nodeCount })
      : plural('graph.notes', counts.nodeCount);

  const failure = rendererError ?? snapshotError;
  const blocked = !workspacePath || failure !== null;

  return (
    <div className={inactive ? 'q-graph q-graph--hidden' : 'q-graph'}>
      {visible && <canvas className="q-graph-canvas" key={attempt} ref={canvasRef} />}
      {!blocked && (
        <GraphStatus
          status={status}
          edgeCount={counts?.edgeCount ?? null}
          drawnEdges={drawnEdges}
          onScaleReadout={attachScaleReadout}
        />
      )}
      {!blocked && createdShare > 0 && visibleNodes === 0 && (
        <GraphNotice
          title={t('graph.filteredOut')}
          actionLabel={t('graph.showAll')}
          onAction={() => setCreatedShare(0)}
        />
      )}
      {blocked && (
        <GraphNotice
          title={workspacePath ? t('graph.notBuilt') : t('graph.noWorkspace')}
          description={failure ?? undefined}
          actionLabel={workspacePath ? t('graph.retry') : undefined}
          onAction={
            workspacePath ? () => setAttempt((current) => current + 1) : undefined
          }
        />
      )}
      {!blocked && (
        <GraphSettings
          controls={controls}
          range={range}
          onChange={patchControls}
          onRefresh={() => setReload((value) => value + 1)}
        />
      )}
    </div>
  );
}
