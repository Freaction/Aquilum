import { invoke } from '@tauri-apps/api/core';
import type { NoteSuggestion, SearchIndexStatus, SearchResponse } from './types';

export function prepareSearchIndex(workspacePath: string): Promise<SearchIndexStatus> {
  return invoke<SearchIndexStatus>('prepare_search_index', { workspacePath });
}

export function searchKnowledgeBase(
  workspacePath: string,
  query: string,
  limit = 50,
): Promise<SearchResponse> {
  return invoke<SearchResponse>('search_knowledge_base', { workspacePath, query, limit });
}

export function getSearchIndexStatus(workspacePath: string): Promise<SearchIndexStatus> {
  return invoke<SearchIndexStatus>('get_search_index_status', { workspacePath });
}

export function suggestNotes(
  workspacePath: string,
  query: string,
  limit = 12,
): Promise<NoteSuggestion[]> {
  return invoke<NoteSuggestion[]>('suggest_notes', { workspacePath, query, limit });
}
