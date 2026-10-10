export interface Backlink {
  path: string;
  title: string;
  offset: number;
}

export interface OutgoingLink {
  target: string;
  title: string;
  path: string | null;
}

export type LinkDisposition = 'current' | 'new-tab';

export interface WikiLinkResolution {
  paths: (string | null)[];
  complete: boolean;
}

export type WikiLinkResolver = (targets: string[]) => Promise<WikiLinkResolution>;
