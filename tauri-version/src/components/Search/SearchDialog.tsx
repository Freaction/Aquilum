import { t } from '../../i18n';
import { useEffect, useRef, useState } from 'react';
import { Search, X } from 'lucide';
import { Icon } from '../Common/Icon';
import { CodeMirrorField, type CodeMirrorFieldRef } from '../Common/CodeMirrorField';
import { Dialog } from '../Common/Dialog';
import { IconButton } from '../Common/IconButton';
import { ScrollArea } from '../Common/ScrollArea';
import { matchesShortcut, SHORTCUTS } from '../../config/shortcuts';
import { useKnowledgeSearch, type SearchResult } from '../../modules/search';
import { SearchContent } from './SearchContent';
import { SearchFooter } from './SearchFooter';
import type { LinkDisposition } from '../../modules/links';
import type { SearchDialogProps } from './types';
import './SearchDialog.css';

export function SearchDialog({
  open,
  workspacePath,
  onClose,
  onOpenResult,
  onCreate,
}: SearchDialogProps) {
  const inputRef = useRef<CodeMirrorFieldRef>(null);
  const resultListRef = useRef<HTMLDivElement>(null);
  const [query, setQuery] = useState('');
  const [selectedIndex, setSelectedIndex] = useState(0);
  const search = useKnowledgeSearch({ open, query, workspacePath });

  useEffect(() => {
    if (open) setSelectedIndex(0);
  }, [open]);

  useEffect(() => setSelectedIndex(0), [query]);
  useEffect(() => {
    setSelectedIndex((current) => Math.min(current, Math.max(search.results.length - 1, 0)));
  }, [search.results.length]);
  useEffect(() => {
    resultListRef.current
      ?.querySelector<HTMLElement>('[aria-selected="true"]')
      ?.scrollIntoView({ block: 'nearest' });
  }, [selectedIndex]);

  if (!open) return null;

  const openResult = (result: SearchResult, disposition: LinkDisposition) => {
    onOpenResult(result, disposition);
    onClose();
  };
  const openSelected = (disposition: LinkDisposition) => {
    const result = search.results[selectedIndex];
    if (result) openResult(result, disposition);
  };
  const createFromQuery = () => {
    const title = query.trim();
    if (!title) return;
    void onCreate(title);
    onClose();
  };
  const handleInputKeyDown = (event: KeyboardEvent) => {
    if (matchesShortcut(event, SHORTCUTS.SEARCH_NEXT)
      || matchesShortcut(event, SHORTCUTS.SEARCH_PREVIOUS)) {
      event.preventDefault();
      event.stopPropagation();
      if (search.results.length === 0) return;
      const direction = matchesShortcut(event, SHORTCUTS.SEARCH_NEXT) ? 1 : -1;
      setSelectedIndex((index) => (
        index + direction + search.results.length
      ) % search.results.length);
    } else if (matchesShortcut(event, SHORTCUTS.SEARCH_CREATE)) {
      event.preventDefault();
      event.stopPropagation();
      createFromQuery();
    } else if (matchesShortcut(event, SHORTCUTS.SEARCH_OPEN_NEW_PANE)) {
      event.preventDefault();
      event.stopPropagation();
      openSelected('new-tab');
    } else if (matchesShortcut(event, SHORTCUTS.SEARCH_OPEN)) {
      event.preventDefault();
      event.stopPropagation();
      openSelected('current');
    } else {
      return false;
    }
    return true;
  };

  return (
    <Dialog
      open
      headerless
      title={t('search.title')}
      className="q-search-dialog"
      initialFocus={() => inputRef.current?.select()}
      onClose={onClose}
    >
      <div className="q-search-box">
        <Icon icon={Search} />
        <CodeMirrorField
          ref={inputRef}
          className="q-search-input"
          value={query}
          onChange={setQuery}
          placeholder={t('search.title')}
          ariaLabel={t('search.title')}
          mode="single-line"
          onKeyDown={handleInputKeyDown}
        />
        {query && (
          <IconButton
            label={t('search.clear')}
            size="small"
            className="q-search-box__clear"
            onClick={() => {
              setQuery('');
              inputRef.current?.focus();
            }}
          >
            <Icon icon={X} />
          </IconButton>
        )}
      </div>
      <ScrollArea
        ref={resultListRef}
        className="q-search-results"
        role="listbox"
        aria-label={t('search.resultsAria')}
      >
        <SearchContent
          {...search}
          query={query}
          selectedIndex={selectedIndex}
          workspacePath={workspacePath}
          onSelectIndex={setSelectedIndex}
          onOpenResult={openResult}
        />
      </ScrollArea>
      <SearchFooter />
    </Dialog>
  );
}
