import { invoke } from '@tauri-apps/api/core';
import type { AnalysisMethod, AnalysisResult } from './types';

export function analyzeDocument(
  workspacePath: string,
  documentPath: string,
  method: AnalysisMethod,
  limit = 50,
): Promise<AnalysisResult[]> {
  return invoke<AnalysisResult[]>('analyze_document', {
    workspacePath,
    documentPath,
    method,
    limit,
  });
}
