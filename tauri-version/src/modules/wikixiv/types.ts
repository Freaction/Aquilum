export interface WikixivHit {
  title: string;
  url: string;
  snippet: string;
  thumbnailUrl?: string;
  fromFilename: boolean;
  matchedTerms: string[];
}

export interface WikixivSearchResult {
  hits: WikixivHit[];
  generation: number;
  offline: boolean;
  insufficientText: boolean;
}
