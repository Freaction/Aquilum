import { File } from 'lucide';
import { Icon } from '../Common/Icon';
import type { SearchResult } from '../../modules/search';
import { highlightMatches } from './highlight';
import './SearchResultCard.css';
import { plural } from '../../i18n';

interface SearchResultCardProps {
  result: SearchResult;
  queryTerms: string[];
  selected: boolean;
  onSelect: (newTab: boolean) => void;
  onHover: () => void;
}

export function SearchResultCard({
  result,
  queryTerms,
  selected,
  onSelect,
  onHover,
}: SearchResultCardProps) {
  return (
    <button
      type="button"
      role="option"
      aria-selected={selected}
      className={`q-search-result ${selected ? 'is-selected' : ''}`}
      onClick={(event) => onSelect(event.ctrlKey || event.metaKey)}
      onMouseEnter={onHover}
      data-result-path={result.path}
    >
      <span className="q-search-result__header">
        <span className="q-search-result__identity">
          <Icon icon={File} className="q-search-result__icon" />
          <span className="q-search-result__filename">
            <span className="q-search-result__title">{highlightMatches(result.title, queryTerms)}</span>
            {result.extension && (
              <span className="q-search-result__extension">
                .{highlightMatches(result.extension, queryTerms)}
              </span>
            )}
          </span>
        </span>
        <span className="q-search-result__count">{formatMatchCount(result.matchCount)}</span>
      </span>
      <span className="q-search-result__snippet">
        {highlightMatches(result.snippet, queryTerms)}
      </span>
    </button>
  );
}

function formatMatchCount(count: number): string {
  return plural('search.matches', count);
}
