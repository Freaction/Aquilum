type SearchIndexState = 'idle' | 'indexing' | 'ready' | 'error';

export interface SearchIndexStatus {
  state: SearchIndexState;
  updating: boolean;
  generation: number;
  revision: number;
  indexedDocuments: number;
  scannedDocuments: number;
  error?: string;
}

export interface SearchResult {
  path: string;
  title: string;
  extension: string;
  snippet: string;
  matchCount: number;
  matchOffset: number;
  heading?: string;
  matchedTerms: string[];
}

export interface SearchResponse {
  queryTerms: string[];
  results: SearchResult[];
  status: SearchIndexStatus;
}

export interface NoteSuggestion {
  path: string;
  title: string;
}
