import { useDocumentQuery } from '../../hooks/useDocumentQuery';
import { analyzeDocument } from './gateway';
import type { AnalysisMethod } from './types';

export function useDocumentAnalysis(
  method: AnalysisMethod,
  workspacePath: string | null,
  documentPath: string | null,
  indexReady: boolean,
  indexRevision: number,
  enabled: boolean,
) {
  const { result, failed, loading } = useDocumentQuery({
    scope: method,
    workspacePath,
    documentPath,
    indexReady,
    indexRevision,
    enabled,
    load: (workspace, document) => analyzeDocument(workspace, document, method),
  });
  return { items: result ?? (failed ? [] : null), failed, loading };
}
