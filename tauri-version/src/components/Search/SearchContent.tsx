import { t } from '../../i18n';
import { DatabaseZap, FolderSearch, Search, SearchX } from 'lucide';
import { formatShortcut, SHORTCUTS } from '../../config/shortcuts';
import { EmptyState } from '../Common/EmptyState';
import type { SearchIndexStatus, SearchResult } from '../../modules/search';
import { SearchResultCard } from './SearchResultCard';
import type { LinkDisposition } from '../../modules/links';

interface SearchContentProps {
  failed: boolean;
  query: string;
  queryTerms: string[];
  results: SearchResult[];
  selectedIndex: number;
  status: SearchIndexStatus;
  workspacePath: string | null;
  onSelectIndex: (index: number) => void;
  onOpenResult: (result: SearchResult, disposition: LinkDisposition) => void;
}

export function SearchContent(props: SearchContentProps) {
  if (!props.workspacePath) {
    return (
      <EmptyState
        icon={FolderSearch}
        title={t('search.noFolder')}
        description={t('search.noFolderHint')}
      />
    );
  }
  if (props.failed || props.status.state === 'error') {
    return (
      <EmptyState
        icon={DatabaseZap}
        title={t('search.unavailable')}
        description={t('search.unavailableHint')}
      />
    );
  }
  if (!props.query.trim()) {
    return (
      <EmptyState
        icon={Search}
        title={t('search.title')}
        description={t('search.titleHint')}
      />
    );
  }
  if (props.results.length === 0 && props.status.state === 'indexing') {
    return (
      <EmptyState
        icon={Search}
        title={t('search.indexing')}
        description={t('search.indexingHint', { count: props.status.scannedDocuments })}
      />
    );
  }
  if (props.results.length === 0) {
    return (
      <EmptyState
        icon={SearchX}
        title={t('search.noResults')}
        description={t('search.createHint', { shortcut: formatShortcut(SHORTCUTS.SEARCH_CREATE) })}
      />
    );
  }
  return props.results.map((result, index) => (
    <SearchResultCard
      key={result.path}
      result={result}
      queryTerms={props.queryTerms}
      selected={index === props.selectedIndex}
      onHover={() => props.onSelectIndex(index)}
      onSelect={(newTab) => {
        props.onSelectIndex(index);
        props.onOpenResult(result, newTab ? 'new-tab' : 'current');
      }}
    />
  ));
}
