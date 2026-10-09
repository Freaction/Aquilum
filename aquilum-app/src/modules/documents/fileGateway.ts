import { invoke } from '@tauri-apps/api/core';

export interface WorkspaceItem {
  id: string;
  name: string;
  type: 'file' | 'folder';
}

export interface FileSnapshot {
  content: string;
  hash: string;
  textHash: string;
}

export interface FileWriteResult {
  hash: string;
}

export interface FileRenameResult extends FileSnapshot {
  updatedPaths: string[];
}

type FileErrorCode =
  | 'io'
  | 'not_found'
  | 'invalid_utf8'
  | 'conflict'
  | 'already_exists'
  | 'task';

interface FileCommandError {
  code: FileErrorCode;
  details: unknown;
}

export function isFileCommandError(
  error: unknown,
  code?: FileErrorCode,
): error is FileCommandError {
  if (typeof error !== 'object' || error === null || !('code' in error)) {
    return false;
  }
  return code === undefined || error.code === code;
}

export function isMarkdownPath(path: string): boolean {
  return path.toLowerCase().endsWith('.md');
}

export function readDirectory(path: string): Promise<WorkspaceItem[]> {
  return invoke<WorkspaceItem[]>('read_directory', { path });
}

export function existingFiles(paths: string[]): Promise<string[]> {
  return invoke<string[]>('existing_files', { paths });
}

export function readFileSnapshot(path: string): Promise<FileSnapshot> {
  return invoke<FileSnapshot>('read_file_snapshot', { path });
}

interface FileStat {
  byteLength: number;
}

export function readFileStat(path: string): Promise<FileStat> {
  return invoke<FileStat>('read_file_stat', { path });
}

export type WriteSource =
  | { kind: 'edit' }
  | { kind: 'readingProgress' }
  | { kind: 'restore'; fromMs: number }
  | { kind: 'revert'; fromMs: number };

export const EDIT_WRITE: WriteSource = { kind: 'edit' };

export function writeFileAtomic(
  path: string,
  content: string,
  expectedHash: string | null = null,
  source: WriteSource = EDIT_WRITE,
): Promise<FileWriteResult> {
  return invoke<FileWriteResult>('write_file_atomic', {
    path,
    content,
    expectedHash,
    source,
  });
}

export function createFile(path: string, content = ''): Promise<FileWriteResult> {
  return invoke<FileWriteResult>('create_file', { path, content });
}

export function createBinaryFile(path: string, bytes: Uint8Array): Promise<FileWriteResult> {
  return invoke<FileWriteResult>('create_binary_file', { path, bytes: Array.from(bytes) });
}

export function renameFile(oldPath: string, newPath: string): Promise<FileRenameResult> {
  return invoke<FileRenameResult>('rename_file', { oldPath, newPath });
}

export function resolveAttachments(
  workspacePath: string,
  names: string[],
): Promise<(string | null)[]> {
  return invoke<(string | null)[]>('resolve_attachments', { workspacePath, names });
}

export function trashFile(workspacePath: string, path: string): Promise<string> {
  return invoke<string>('trash_file', { workspacePath, path });
}

export function ensureDirectory(path: string): Promise<void> {
  return invoke<void>('ensure_directory', { path });
}

export function copyFile(from: string, to: string): Promise<void> {
  return invoke<void>('copy_file', { from, to });
}

const FREE_NAME_ATTEMPTS = 10_000;

export async function createAtFreeName(
  pathFor: (attempt: number) => string,
  create: (path: string) => Promise<unknown>,
): Promise<string | null> {
  for (let attempt = 0; attempt < FREE_NAME_ATTEMPTS; attempt += 1) {
    const path = pathFor(attempt);
    try {
      await create(path);
      return path;
    } catch (error) {
      if (!isFileCommandError(error, 'already_exists')) throw error;
    }
  }
  return null;
}
