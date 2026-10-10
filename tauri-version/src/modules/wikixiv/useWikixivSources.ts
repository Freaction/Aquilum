import { useCallback, useEffect, useRef, useState } from 'react';
import { onDocumentChanged, readDocument } from '../documents/documentGateway';
import { isEmptyTabPath } from '../ui-state';
import { resolveDocumentText } from './documentText';
import { searchWikixiv } from './gateway';
import { WIKIXIV_PAUSE_MS, looksLikeParagraphBreak } from './searchTriggers';
import type { WikixivHit, WikixivSearchResult } from './types';

interface HookState {
  hits: WikixivHit[];
  offline: boolean;
  insufficientText: boolean;
  failed: boolean;
  loading: boolean;
}

function emptyState(): HookState {
  return { hits: [], offline: false, insufficientText: false, failed: false, loading: false };
}

export function useWikixivSources(documentPath: string | null, enabled: boolean) {
  const [state, setState] = useState<HookState>(emptyState);
  const generationRef = useRef(0);
  const lastTextRef = useRef('');
  const pauseTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const documentPathRef = useRef(documentPath);
  documentPathRef.current = documentPath;

  const clearPauseTimer = () => {
    if (pauseTimerRef.current) {
      clearTimeout(pauseTimerRef.current);
      pauseTimerRef.current = null;
    }
  };

  const runSearch = useCallback(async (text: string) => {
    const generation = generationRef.current + 1;
    generationRef.current = generation;
    setState((current) => (
      current.hits.length > 0
        ? current
        : { ...current, loading: true, failed: false }
    ));
    try {
      const result: WikixivSearchResult = await searchWikixiv(
        text,
        generation,
        documentPathRef.current,
      );
      if (generationRef.current !== result.generation) return;
      setState({
        hits: result.hits,
        offline: result.offline,
        insufficientText: result.insufficientText,
        failed: false,
        loading: false,
      });
    } catch (error) {
      if (generationRef.current !== generation) return;
      console.error('Wikixiv search failed', error);
      setState((current) => ({ ...current, failed: true, loading: false }));
    }
  }, []);

  useEffect(() => {
    if (!enabled) {
      clearPauseTimer();
      generationRef.current += 1;
      setState(emptyState());
      lastTextRef.current = '';
      return;
    }

    let cancelled = false;
    void (async () => {
      const text = await resolveDocumentText(documentPathRef.current);
      if (cancelled) return;
      lastTextRef.current = text;
      await runSearch(text);
    })();

    return () => {
      cancelled = true;
      clearPauseTimer();
      generationRef.current += 1;
    };
  }, [documentPath, enabled, runSearch]);

  useEffect(() => {
    if (!enabled || !documentPath || isEmptyTabPath(documentPath)) return;
    let disposed = false;

    const onUpdate = async () => {
      const { text } = await readDocument(documentPath);
      if (disposed) return;
      const previous = lastTextRef.current;
      lastTextRef.current = text;
      clearPauseTimer();
      if (looksLikeParagraphBreak(previous, text)) {
        void runSearch(text);
        return;
      }
      pauseTimerRef.current = setTimeout(() => void runSearch(text), WIKIXIV_PAUSE_MS);
    };

    const unsubscribe = onDocumentChanged(() => documentPath, () => {
      void onUpdate().catch((error) => console.error('Failed to read the note for wikixiv', error));
    });
    return () => {
      disposed = true;
      unsubscribe();
      clearPauseTimer();
    };
  }, [documentPath, enabled, runSearch]);

  return state;
}
