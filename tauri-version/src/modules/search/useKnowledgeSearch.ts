import { useCallback, useEffect, useRef, useState } from 'react';
import { getSearchIndexStatus, prepareSearchIndex, searchKnowledgeBase } from './gateway';
import type { SearchIndexStatus, SearchResult } from './types';

const IDLE_STATUS: SearchIndexStatus = {
  state: 'idle',
  updating: false,
  generation: 0,
  revision: 0,
  indexedDocuments: 0,
  scannedDocuments: 0,
};

interface KnowledgeSearchOptions {
  open: boolean;
  query: string;
  workspacePath: string | null;
}

export function useKnowledgeSearch({
  open,
  query,
  workspacePath,
}: KnowledgeSearchOptions) {
  const workspaceGenerationRef = useRef(0);
  const searchRequestRef = useRef(0);
  const [status, setStatus] = useState<SearchIndexStatus>(IDLE_STATUS);
  const [results, setResults] = useState<SearchResult[]>([]);
  const [queryTerms, setQueryTerms] = useState<string[]>([]);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    workspaceGenerationRef.current += 1;
    searchRequestRef.current += 1;
    setStatus(IDLE_STATUS);
    setResults([]);
    setQueryTerms([]);
    setFailed(false);
  }, [workspacePath]);

  useEffect(() => {
    if (!open || !workspacePath) return;
    const generation = workspaceGenerationRef.current;
    void prepareSearchIndex(workspacePath)
      .then((nextStatus) => {
        if (generation === workspaceGenerationRef.current) setStatus(nextStatus);
      })
      .catch((error) => {
        if (generation !== workspaceGenerationRef.current) return;
        console.error('Failed to prepare search index', error);
        setFailed(true);
      });
  }, [open, workspacePath]);

  useEffect(() => {
    if (!open || !workspacePath || !status.updating) return;
    const generation = workspaceGenerationRef.current;
    let polling = false;
    const interval = window.setInterval(() => {
      if (polling) return;
      polling = true;
      void getSearchIndexStatus(workspacePath)
        .then((nextStatus) => {
          if (generation === workspaceGenerationRef.current) setStatus(nextStatus);
        })
        .catch((error) => console.error('Failed to read search index status', error))
        .finally(() => { polling = false; });
    }, 500);
    return () => window.clearInterval(interval);
  }, [open, status.updating, workspacePath]);

  const runSearch = useCallback(async (requestId: number, workspace: string, value: string) => {
    try {
      const response = await searchKnowledgeBase(workspace, value);
      if (requestId !== searchRequestRef.current) return;
      setResults(response.results);
      setQueryTerms(response.queryTerms);
      setStatus(response.status);
      setFailed(false);
    } catch (error) {
      if (requestId !== searchRequestRef.current) return;
      console.error('Knowledge base search failed', error);
      setFailed(true);
    }
  }, []);

  useEffect(() => {
    if (!open) {
      ++searchRequestRef.current;
      return;
    }
    if (!workspacePath || status.state === 'idle' || status.state === 'error') return;
    const value = query.trim();
    const requestId = ++searchRequestRef.current;
    if (!value) {
      setResults([]);
      setQueryTerms([]);
      setFailed(false);
      return;
    }
    const timeout = window.setTimeout(() => void runSearch(requestId, workspacePath, value), 70);
    return () => window.clearTimeout(timeout);
  }, [
    open,
    query,
    runSearch,
    status.indexedDocuments,
    status.state,
    workspacePath,
  ]);

  return { failed, queryTerms, results, status };
}
