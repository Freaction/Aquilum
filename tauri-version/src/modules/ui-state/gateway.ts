import { invoke } from '@tauri-apps/api/core';
import type { LoadedSession, ViewState, OpenSessionInput, SaveStateBatchInput } from './types';

export function resolveWorkspace(path: string, nowMs: number): Promise<string> {
  return invoke<string>('resolve_ui_workspace', { path, nowMs });
}

export function resolveDocument(workspaceId: string, relativePath: string): Promise<string> {
  return invoke<string>('resolve_ui_document', { workspaceId, relativePath });
}

export function openSession(input: OpenSessionInput): Promise<LoadedSession> {
  return invoke<LoadedSession>('open_ui_session', { input });
}

export function saveStateBatch(input: SaveStateBatchInput): Promise<boolean> {
  return invoke<boolean>('save_ui_state_batch', { input });
}

export function renameDocument(documentId: string, relativePath: string): Promise<void> {
  return invoke<void>('rename_ui_document', { documentId, relativePath });
}

export function markDocumentMissing(documentId: string, nowMs: number): Promise<void> {
  return invoke<void>('mark_ui_document_missing', { documentId, nowMs });
}

function resetUiState(nowMs: number): Promise<void> {
  return invoke<void>('reset_ui_state', { nowMs });
}

export async function resetSessionState(workspacePath: string | null): Promise<void> {
  const now = Date.now();
  await resetUiState(now);
  if (workspacePath) await resolveWorkspace(workspacePath, now);
  window.location.reload();
}

export function cleanupUiState(retentionDays: number, nowMs: number): Promise<number> {
  return invoke<number>('cleanup_ui_state', { retentionDays, nowMs });
}

export function loadDocumentView(
  workspaceId: string,
  windowId: string,
  documentId: string,
  paneId: string,
): Promise<ViewState | null> {
  return invoke<ViewState | null>('load_ui_document_view', {
    workspaceId,
    windowId,
    documentId,
    paneId,
  });
}

interface ReaderStateDto {
  current: number;
  cfi?: string | null;
}

export function loadReaderState(
  workspaceId: string,
  bookFile: string,
): Promise<ReaderStateDto | null> {
  return invoke<ReaderStateDto | null>('load_ui_reader_state', { workspaceId, bookFile });
}

export function saveReaderState(input: {
  workspaceId: string;
  bookFile: string;
  current: number;
  cfi?: string | null;
  nowMs: number;
}): Promise<void> {
  return invoke<void>('save_ui_reader_state', { input });
}
