import type { LinkDisposition } from '../../modules/links';
import type { SearchResult } from '../../modules/search';

export interface SearchDialogProps {
  open: boolean;
  workspacePath: string | null;
  onClose: () => void;
  onOpenResult: (result: SearchResult, disposition: LinkDisposition) => void;
  onCreate: (title: string) => void | Promise<void>;
}
