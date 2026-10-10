import { invoke } from '@tauri-apps/api/core';

export type DataviewCellPart =
  | { kind: 'text'; text: string }
  | { kind: 'link'; target: string; text: string }
  | { kind: 'progress'; percent: number }
  | { kind: 'check'; done: boolean; target: string; line: number };

export interface DataviewCell {
  parts: DataviewCellPart[];
}

export interface DataviewOutput {
  shape: 'table' | 'list' | 'tasks';
  refreshSeconds: number | null;
  title: string | null;
  columns: string[];
  rows: DataviewCell[];
  width: number;
  total: number;
  truncated: boolean;
}

export function runDataviewQuery(
  workspacePath: string,
  documentPath: string,
  query: string,
): Promise<DataviewOutput> {
  return invoke<DataviewOutput>('run_dataview_query', {
    workspacePath,
    documentPath,
    query,
    tzOffsetMinutes: -new Date().getTimezoneOffset(),
  });
}

export function dataviewErrorMessage(error: unknown): string {
  if (typeof error === 'string') return error;
  const message = (error as { details?: { message?: string } } | null)?.details?.message;
  return message ?? 'Запрос не удалось выполнить';
}
