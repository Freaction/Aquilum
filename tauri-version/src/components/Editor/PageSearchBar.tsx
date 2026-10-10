import { useEffect, useRef, useState } from 'react';
import { t } from '../../i18n';
import { matchesShortcut, SHORTCUTS } from '../../config/shortcuts';
import type { EditorView } from '@codemirror/view';
import { ArrowDown, ArrowUp, Search, TextSelect, X } from 'lucide';
import { Icon } from '../Common/Icon';
import type { CodeMirrorFieldRef } from '../Common/CodeMirrorField';
import { IconButton } from '../Common/IconButton';
import { Input } from '../Common/Input';
import './PageSearchBar.css';
import { type PageSearchSnapshot, updatePageSearch } from './extensions/pageSearch';

interface PageSearchBarProps {
  focusRequest: number;
  open: boolean;
  view: EditorView | null;
  onClose: () => void;
}

const EMPTY_SEARCH: PageSearchSnapshot = { current: 0, total: 0, activeOnly: false };

export function PageSearchBar({ focusRequest, open, view, onClose }: PageSearchBarProps) {
  const [query, setQuery] = useState('');
  const [search, setSearch] = useState<PageSearchSnapshot>(EMPTY_SEARCH);
  const inputRef = useRef<CodeMirrorFieldRef>(null);

  useEffect(() => {
    if (open) requestAnimationFrame(() => inputRef.current?.focus());
  }, [focusRequest, open]);

  useEffect(() => {
    if (!view) return;
    setSearch(updatePageSearch(view, open ? query : '', 0, false));
  }, [open, query, view]);

  if (!open) return null;
  const navigate = (delta: -1 | 1) => {
    if (!view || !query) return;
    setSearch(updatePageSearch(view, query, Math.max(search.current - 1, 0) + delta, true));
  };
  const showAllMatches = () => {
    if (!view || !query || !search.total) return;
    setSearch(updatePageSearch(view, query, Math.max(search.current - 1, 0), false));
  };
  const close = () => {
    if (view) {
      updatePageSearch(view, '', 0, false);
      view.focus();
    }
    setQuery('');
    setSearch(EMPTY_SEARCH);
    onClose();
  };

  return (
    <div className="q-page-search" role="search">
      <div className="q-page-search__field">
        <Input
          fullWidth
          counter={`${search.current}/${search.total}`}
          startAdornment={<Icon icon={Search} />}
          ref={inputRef}
          value={query}
          placeholder={t('editor.find')}
          ariaLabel={t('editor.findAria')}
          onChange={setQuery}
          onKeyDown={(event) => {
            if (event.key === 'Escape') { event.preventDefault(); close(); }
            if (matchesShortcut(event, SHORTCUTS.PAGE_SEARCH_NEXT)) { event.preventDefault(); navigate(1); }
            if (matchesShortcut(event, SHORTCUTS.PAGE_SEARCH_PREVIOUS)) { event.preventDefault(); navigate(-1); }
            return event.defaultPrevented;
          }}
        />
      </div>
      <div className="q-page-search__actions">
        <IconButton size="medium" label={t('editor.prevMatch')} onClick={() => navigate(-1)} disabled={!query}><Icon icon={ArrowUp} /></IconButton>
        <IconButton size="medium" label={t('editor.nextMatch')} onClick={() => navigate(1)} disabled={!query}><Icon icon={ArrowDown} /></IconButton>
        <IconButton size="medium" label={t('editor.selectAllMatches')} onClick={showAllMatches} disabled={!search.activeOnly}><Icon icon={TextSelect} /></IconButton>
        <IconButton size="medium" label={t('editor.closeSearch')} onClick={close}><Icon icon={X} /></IconButton>
      </div>
    </div>
  );
}

