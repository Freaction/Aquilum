import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';
import type { EditorView, ViewUpdate } from '@codemirror/view';
import { DOCUMENT_HEAD_LIMIT, documentHead } from './useEditorDocContent';
import { ViewStateController } from '../viewState/controller';
import { fallbackView, initialSelection, type ResolvedView } from '../viewState/positions';
import { resolvePositions } from '../../../modules/documents/documentGateway';
import { resolveEditorScrollElement } from '../viewState/scroll';
import { markOpenStage } from '../../../modules/perf/openTrace';
import type { ViewState } from '../../../modules/ui-state';

export function useEditorViewSetup(options: {
  documentId: string | null;
  viewStateReady: boolean;
  initialViewState: ViewState | null;
  revealOffset?: number;
  path: () => string;
  isReady: boolean;
  initialBody: string;
  onViewStateChange: (state: ViewState) => void;
  setDocContent: (content: string) => void;
  inactive?: boolean;
}) {
  const containerRef = useRef<HTMLDivElement>(null);
  const {
    documentId,
    viewStateReady,
    initialViewState,
    revealOffset,
    path,
    isReady,
    initialBody,
    onViewStateChange,
    setDocContent,
    inactive = false,
  } = options;
  const onViewStateChangeRef = useRef(onViewStateChange);
  onViewStateChangeRef.current = onViewStateChange;
  const handleViewStateChange = useCallback((state: ViewState) => {
    onViewStateChangeRef.current(state);
  }, []);

  const resolved = useResolvedView(isReady && viewStateReady, initialViewState, path);

  // The saved view state only seeds a new controller; recreating the controller when the parent
  // passes a fresh copy of it would re-run restore() on a hidden tab and lose its selection.
  const initialViewStateRef = useRef(initialViewState);
  initialViewStateRef.current = initialViewState;
  const pathRef = useRef(path);
  pathRef.current = path;
  const currentPath = useCallback(() => pathRef.current(), []);

  const controller = useMemo(
    () => (isReady && documentId && viewStateReady && resolved !== undefined
      ? new ViewStateController({
        documentId,
        path: currentPath,
        initial: initialViewStateRef.current,
        resolved,
        onChange: handleViewStateChange,
        revealOffset,
      })
      : null),
    [currentPath, documentId, handleViewStateChange, isReady, resolved, revealOffset, viewStateReady],
  );

  const selection = useMemo(
    () => initialSelection(resolved ?? null, initialBody.length, revealOffset),
    [initialBody.length, resolved, revealOffset],
  );

  const viewRef = useRef<EditorView | null>(null);
  const controllerRef = useRef<ViewStateController | null>(null);
  const attachedRef = useRef<ViewStateController | null>(null);
  controllerRef.current = controller;

  const attachWhenReady = useCallback(() => {
    const view = viewRef.current;
    const pending = controllerRef.current;
    const container = containerRef.current;
    if (!view || !pending || !container) return;
    if (attachedRef.current === pending) return;
    attachedRef.current = pending;
    pending.attach(view, resolveEditorScrollElement(container));
  }, []);

  useLayoutEffect(() => () => {
    controller?.dispose(true);
    if (attachedRef.current === controller) attachedRef.current = null;
  }, [controller]);

  useEffect(attachWhenReady, [attachWhenReady, controller]);

  const wasInactive = useRef(inactive);
  useEffect(() => {
    const shown = wasInactive.current && !inactive;
    wasInactive.current = inactive;
    if (!shown) return;
    const view = viewRef.current;
    if (!view) return;
    controllerRef.current?.restoreNow();
    // Focusing the pane before the browser has shown it lets the browser drop the caret at the start
    // of the note, and CodeMirror adopts that as its selection.
    const frame = requestAnimationFrame(() => view.focus());
    markOpenStage('mount');
    return () => cancelAnimationFrame(frame);
  }, [inactive]);


  const handleCreate = useCallback((view: EditorView) => {
    viewRef.current = view;
    markOpenStage('mount');
    attachWhenReady();
  }, [attachWhenReady]);

  const handleUpdate = useCallback((update: ViewUpdate) => {
    controller?.update(update);
    if (update.docChanged && update.changes.touchesRange(0, DOCUMENT_HEAD_LIMIT)) {
      setDocContent(documentHead(update.state.doc));
    }
  }, [controller, setDocContent]);

  const ready = isReady && viewStateReady && resolved !== undefined;

  return {
    containerRef,
    selection,
    handleCreate,
    handleUpdate,
    ready,
  };
}

function useResolvedView(
  enabled: boolean,
  initial: ViewState | null,
  path: () => string,
): ResolvedView | null | undefined {
  const [resolved, setResolved] = useState<ResolvedView | null | undefined>(undefined);
  const pathRef = useRef(path);
  pathRef.current = path;

  useEffect(() => {
    if (!enabled || resolved !== undefined) return;
    if (!initial) {
      setResolved(null);
      return;
    }
    let cancelled = false;
    const fallback = fallbackView(initial);
    resolvePositions(pathRef.current(), [initial.cursorAnchor, initial.cursorHead, initial.scrollAnchor])
      .then(([anchor, head, scroll]) => {
        if (cancelled) return;
        setResolved({
          anchor: anchor ?? fallback.anchor,
          head: head ?? fallback.head,
          scroll: scroll ?? fallback.scroll,
        });
      })
      .catch((error: unknown) => {
        console.error('Failed to resolve the saved view positions', error);
        if (!cancelled) setResolved(fallback);
      });
    return () => {
      cancelled = true;
    };
  }, [enabled, initial, resolved]);

  return resolved;
}
