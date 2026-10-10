import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { samePath } from '../paths';
import type { WriteSource } from './fileGateway';
import { legacyReplicasImported } from './legacyReplicas';

export interface OpenedDocument {
  text: string;
  version: number;
}

export interface LoggedChange {
  client: string;
  changes: unknown;
}

export type PullResult =
  | { kind: 'changes'; changes: LoggedChange[] }
  | { kind: 'resync'; text: string; version: number };

interface DocumentEvent {
  path: string;
  version: number;
}

interface SaveFailed {
  path: string;
  reason: string;
}

const MAX_STALE_RETRIES = 5;

function timezoneOffsetMinutes(): number {
  return -new Date().getTimezoneOffset();
}

export async function openDocument(path: string): Promise<OpenedDocument> {
  await legacyReplicasImported();
  return invoke<OpenedDocument>('document_open', { path, tzOffsetMinutes: timezoneOffsetMinutes() });
}

export async function readDocument(path: string): Promise<OpenedDocument> {
  await legacyReplicasImported();
  return invoke<OpenedDocument>('document_read', { path });
}

export function releaseDocument(path: string): Promise<void> {
  return invoke('document_release', { path });
}

export function pushChanges(path: string, version: number, client: string, changes: unknown[]): Promise<boolean> {
  return invoke<boolean>('document_push', { path, version, client, changes });
}

export function pullChanges(path: string, version: number): Promise<PullResult> {
  return invoke<PullResult>('document_pull', { path, version });
}

export function resolvePositions(path: string, positions: number[][]): Promise<(number | null)[]> {
  return invoke<(number | null)[]>('document_resolve_positions', { path, positions });
}

export async function revertDocument(
  path: string,
  versionText: string,
  previousText: string,
  fromMs: number,
): Promise<boolean> {
  await legacyReplicasImported();
  return invoke<boolean>('document_revert', { path, versionText, previousText, fromMs });
}

export function flushDocuments(): Promise<void> {
  return invoke('document_flush_all');
}

export function reconcileDocuments(): Promise<void> {
  return invoke('document_reconcile_all');
}

function hasCode(error: unknown, code: string): boolean {
  return typeof error === 'object' && error !== null && 'code' in error && error.code === code;
}

export function isDocumentMissing(error: unknown): boolean {
  return hasCode(error, 'missing');
}

export async function rewriteDocument(
  path: string,
  rewrite: (text: string) => string | null,
  source?: WriteSource,
): Promise<boolean> {
  for (let attempt = 0; attempt < MAX_STALE_RETRIES; attempt += 1) {
    const { text, version } = await readDocument(path);
    const next = rewrite(text);
    if (next === null || next === text) return false;
    try {
      await invoke<number>('document_replace_text', { path, text: next, source, baseVersion: version });
      return true;
    } catch (error) {
      if (!hasCode(error, 'stale')) throw error;
    }
  }
  throw new Error(`Документ ${path} всё время меняется, правка не применена`);
}

function onPathEvent<T extends { path: string }>(
  event: string,
  path: () => string,
  handler: (payload: T) => void,
): () => void {
  let disposed = false;
  let unlisten: (() => void) | null = null;
  void listen<T>(event, ({ payload }) => {
    if (samePath(payload.path, path())) handler(payload);
  }).then((dispose) => {
    if (disposed) dispose();
    else unlisten = dispose;
  });
  return () => {
    disposed = true;
    unlisten?.();
  };
}

export function onDocumentChanged(path: () => string, handler: (version: number) => void): () => void {
  return onPathEvent<DocumentEvent>('document-changed', path, ({ version }) => handler(version));
}

export function onDocumentSaved(path: () => string, handler: () => void): () => void {
  return onPathEvent<DocumentEvent>('document-saved', path, () => handler());
}

export function onDocumentSaveFailed(path: () => string, handler: (reason: string) => void): () => void {
  return onPathEvent<SaveFailed>('document-save-failed', path, ({ reason }) => handler(reason));
}

export function onDocumentMissing(path: () => string, handler: () => void): () => void {
  return onPathEvent<DocumentEvent>('document-missing', path, () => handler());
}
