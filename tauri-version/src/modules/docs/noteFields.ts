import { invoke } from '@tauri-apps/api/core';
import type { BookMetadata } from './frontmatter';

interface NoteFieldsEntry {
  path: string;
  fields: BookMetadata;
}

export async function readNoteFields(
  workspacePath: string,
  documentPaths: string[],
): Promise<(BookMetadata | null)[]> {
  if (documentPaths.length === 0) return [];
  const entries = await invoke<NoteFieldsEntry[]>('get_note_fields', {
    workspacePath,
    documentPaths,
  });
  return documentPaths.map((_, index) => entries[index]?.fields ?? null);
}
