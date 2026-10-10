import type { ViewState } from '../../modules/ui-state';
import type { LinkDisposition, WikiLinkResolver } from '../../modules/links';
import type { EditorView } from '@codemirror/view';
import {
  FM_BOOK_COVER,
  FM_BOOK_FILE,
  FM_PAGE_COVER,
  FM_PAGE_COVER_POSITION,
  FM_READ_PERCENT,
  FM_READER_POSITION,
} from '../../modules/docs/frontmatter';

export interface EditorProps {
  filePath: string;
  documentId: string | null;
  workspacePath: string | null;
  initialViewState: ViewState | null;
  viewStateReady: boolean;
  onViewStateChange: (state: ViewState) => void;
  onFileMissing?: (path: string) => void;
  resolveWikiLinks: WikiLinkResolver;
  linkRevision: number;
  onOpenWikiLink: (target: string, disposition: LinkDisposition) => void;
  onOpenExternalUrl: (url: string) => void;
  revealOffset?: number;
  inactive?: boolean;
  onBodyViewChange?: (view: EditorView | null) => void;
}

export type ReaderSession = {
  bookFile: string;
  title: string;
  pagesFm?: string;
  initialCfi?: string;
  peek?: boolean;
  progressPagePath?: string;
};

export type CoverFieldKey =
  | 'cover'
  | 'pages'
  | typeof FM_BOOK_COVER
  | typeof FM_PAGE_COVER
  | typeof FM_PAGE_COVER_POSITION
  | typeof FM_BOOK_FILE
  | typeof FM_READER_POSITION
  | typeof FM_READ_PERCENT;

