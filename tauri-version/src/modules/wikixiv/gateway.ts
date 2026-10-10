import { invoke } from '@tauri-apps/api/core';
import type { WikixivSearchResult } from './types';

export function searchWikixiv(
  text: string,
  generation: number,
  documentPath?: string | null,
): Promise<WikixivSearchResult> {
  return invoke<WikixivSearchResult>('wikixiv_search', {
    text,
    generation,
    documentPath: documentPath ?? null,
  });
}
