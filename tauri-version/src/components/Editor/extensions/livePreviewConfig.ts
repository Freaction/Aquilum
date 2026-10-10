import { Facet, type EditorState } from '@codemirror/state';
import type { LinkDisposition, WikiLinkResolver } from '../../../modules/links';

export type BookCalloutReadRequest = {
  bookFile: string;
  title: string;
  pagesFm?: string;
  bookPagePath?: string;
};

export type LivePreviewConfig = {
  resolveWikiLinks: WikiLinkResolver;
  workspacePath: string | null;
  notePath: () => string;
  onOpenWikiLink: (target: string, disposition: LinkDisposition) => void;
  onOpenExternalUrl: (url: string) => void;
  onReadBook: (request: BookCalloutReadRequest) => void;
};

export const livePreviewConfigFacet = Facet.define<LivePreviewConfig, LivePreviewConfig | null>({
  combine: (values) => values[values.length - 1] ?? null,
});

export const revealAtCaretFacet = Facet.define<boolean, boolean>({
  combine: values => values.every(Boolean),
});

export function previewCaret(state: EditorState): number {
  return state.facet(revealAtCaretFacet) ? state.selection.main.head : -1;
}
