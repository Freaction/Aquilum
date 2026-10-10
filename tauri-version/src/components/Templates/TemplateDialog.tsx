import { t } from '../../i18n';
import { useEffect, useMemo, useRef, useState } from 'react';
import { ArrowUpDown, CornerDownLeft, FileText, Search } from 'lucide';
import { Icon } from '../Common/Icon';
import { CodeMirrorField, type CodeMirrorFieldRef } from '../Common/CodeMirrorField';
import { Dialog, DialogFooter } from '../Common/Dialog';
import { EmptyState } from '../Common/EmptyState';
import { ScrollArea } from '../Common/ScrollArea';
import { TextButton } from '../Common/TextButton';
import { highlightMatches } from '../Search/highlight';
import { KeyHint } from '../Search/SearchFooter';
import { SHORTCUTS } from '../../config/shortcuts';
import { createStarterTemplates, listTemplates, templatesDirectoryExists, templatesFolderName, type NoteTemplate } from '../../modules/templates';
import './TemplateDialog.css';

interface Props {
  open: boolean;
  workspacePath: string | null;
  folder: string;
  onClose: () => void;
  onSelect: (template: NoteTemplate) => void | Promise<void>;
}

interface TemplateListing {
  templates: NoteTemplate[];
  directoryExists: boolean | null;
}

const EMPTY_LISTING: TemplateListing = { templates: [], directoryExists: null };

function useTemplateListing(open: boolean, workspacePath: string | null, folder: string) {
  const [listing, setListing] = useState<TemplateListing>(EMPTY_LISTING);
  const [revision, setRevision] = useState(0);

  useEffect(() => {
    if (!open) return;
    if (!workspacePath) {
      setListing({ templates: [], directoryExists: false });
      return;
    }
    let active = true;
    setListing(EMPTY_LISTING);
    void Promise.all([listTemplates(workspacePath, folder), templatesDirectoryExists(workspacePath, folder)])
      .then(([templates, directoryExists]) => {
        if (active) setListing({ templates, directoryExists });
      });
    return () => { active = false; };
  }, [open, workspacePath, folder, revision]);

  const createStarter = () => {
    if (!workspacePath) return;
    createStarterTemplates(workspacePath, folder)
      .then(() => setRevision((value) => value + 1))
      .catch((error) => console.error('Failed to create starter templates', error));
  };

  return { ...listing, createStarter };
}

function TemplateName({ relativePath, query }: { relativePath: string; query: string }) {
  const name = relativePath.replace(/\.md$/i, '');
  return (
    <>
      {highlightMatches(name, [query], { wordStartOnly: false })}
      <span className="q-template-result__extension">.md</span>
    </>
  );
}

export function TemplateDialog({ open, workspacePath, folder, onClose, onSelect }: Props) {
  const inputRef = useRef<CodeMirrorFieldRef>(null);
  const resultListRef = useRef<HTMLDivElement>(null);
  const selectingRef = useRef(false);
  const [query, setQuery] = useState('');
  const [selected, setSelected] = useState(0);
  const { templates, directoryExists, createStarter } = useTemplateListing(open, workspacePath, folder);

  useEffect(() => {
    if (!open) return;
    setQuery('');
    setSelected(0);
  }, [open]);

  const trimmedQuery = query.trim();
  const visible = useMemo(() => {
    const needle = trimmedQuery.toLocaleLowerCase();
    return templates.filter((item) => item.relativePath.toLocaleLowerCase().includes(needle));
  }, [trimmedQuery, templates]);

  useEffect(() => setSelected(0), [query]);
  useEffect(() => {
    resultListRef.current
      ?.querySelector<HTMLElement>('[aria-selected="true"]')
      ?.scrollIntoView({ block: 'nearest' });
  }, [selected]);

  const choose = (item = visible[selected]) => {
    if (!item || selectingRef.current) return;
    selectingRef.current = true;
    void Promise.resolve(onSelect(item))
      .catch((error) => console.error('Failed to apply a template', error))
      .finally(() => { selectingRef.current = false; });
  };

  const handleInputKeyDown = (event: KeyboardEvent) => {
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      if (visible.length === 0) return true;
      const direction = event.key === 'ArrowDown' ? 1 : -1;
      setSelected((value) => (value + direction + visible.length) % visible.length);
      return true;
    }
    if (event.key === 'Enter') {
      event.preventDefault();
      choose();
      return true;
    }
    return false;
  };

  return (
    <Dialog
      open={open}
      headerless
      title={t('templateDialog.title')}
      className="q-search-dialog q-template-dialog"
      overlayClassName="q-template-overlay"
      initialFocus={() => inputRef.current?.focus()}
      onClose={onClose}
    >
      <div className="q-search-box">
        <Icon icon={Search} />
        <CodeMirrorField
          ref={inputRef}
          className="q-search-input"
          value={query}
          onChange={setQuery}
          placeholder={t('templateDialog.find')}
          ariaLabel={t('templateDialog.find')}
          mode="single-line"
          onKeyDown={handleInputKeyDown}
        />
      </div>
      <ScrollArea ref={resultListRef} className="q-search-results" role="listbox" aria-label={t('templateDialog.title')}>
        {visible.length ? visible.map((item, index) => (
          <button
            key={item.path}
            type="button"
            role="option"
            aria-selected={index === selected}
            className={`q-template-result${index === selected ? ' is-selected' : ''}`}
            onMouseEnter={() => setSelected(index)}
            onClick={() => choose(item)}
          >
            <Icon icon={FileText} />
            <span className="q-template-result__name">
              <TemplateName relativePath={item.relativePath} query={trimmedQuery} />
            </span>
          </button>
        )) : (
          <EmptyState
            icon={FileText}
            title={t('templateDialog.empty')}
            description={t('templateDialog.emptyHint', { folder: templatesFolderName(folder) })}
          >
            {workspacePath && directoryExists === false && (
              <TextButton onClick={createStarter}>{t('templateDialog.createFolder')}</TextButton>
            )}
          </EmptyState>
        )}
      </ScrollArea>
      <DialogFooter align="center">
        <KeyHint shortcut={SHORTCUTS.SEARCH_NEXT} label={t('search.navigate')}><Icon icon={ArrowUpDown} /></KeyHint>
        <KeyHint shortcut={SHORTCUTS.SEARCH_OPEN} label={t('search.open')}><Icon icon={CornerDownLeft} /></KeyHint>
        <KeyHint shortcut={SHORTCUTS.CLOSE_DIALOG} label={t('templateDialog.close')} />
      </DialogFooter>
    </Dialog>
  );
}
