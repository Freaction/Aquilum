import { useDocumentQuery } from '../../hooks/useDocumentQuery';
import { getBacklinks, getOutgoingLinks } from './gateway';
import type { Backlink, OutgoingLink } from './types';

export type LinkMode = 'backlinks' | 'outgoing';

export type DocumentLinks =
  | { mode: 'backlinks'; items: Backlink[] }
  | { mode: 'outgoing'; items: OutgoingLink[] };

const EMPTY_RESULTS: Record<LinkMode, DocumentLinks> = {
  backlinks: { mode: 'backlinks', items: [] },
  outgoing: { mode: 'outgoing', items: [] },
};

function loadLinks(mode: LinkMode, workspacePath: string, documentPath: string): Promise<DocumentLinks> {
  return mode === 'backlinks'
    ? getBacklinks(workspacePath, documentPath).then((items) => ({ mode, items }))
    : getOutgoingLinks(workspacePath, documentPath).then((items) => ({ mode, items }));
}

export function useDocumentLinks(
  mode: LinkMode,
  workspacePath: string | null,
  documentPath: string | null,
  indexReady: boolean,
  indexRevision: number,
  enabled: boolean,
) {
  const { result, failed, loading } = useDocumentQuery({
    scope: mode,
    workspacePath,
    documentPath,
    indexReady,
    indexRevision,
    enabled,
    load: (workspace, document) => loadLinks(mode, workspace, document),
  });
  return { result: result ?? (failed ? EMPTY_RESULTS[mode] : null), loading };
}
