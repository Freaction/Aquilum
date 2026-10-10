import { useEffect, useId, useMemo, useRef, useState } from 'react';
import { readDirectory } from '../../../modules/documents/fileGateway';
import { relativePath } from '../../../modules/paths';
import { t } from '../../../i18n';
import '../../Common/Input.css';
import './DailyNotesFolderPicker.css';

interface Props { workspacePath: string | null; value: string; onChange: (value: string) => void; }
export function DailyNotesFolderPicker({ workspacePath, value, onChange }: Props) {
  const id = useId();
  const identity = useMemo(() => ({}), [workspacePath]);
  const context = useRef<object | null>(identity);
  context.current = identity;
  const cache = useRef<object | null>(null);
  const [folders, setFolders] = useState<string[]>([]);
  const [query, setQuery] = useState(value);
  const [open, setOpen] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState(false);
  const [active, setActive] = useState(-1);
  const dirty = useRef(false);
  const list = useRef<HTMLDivElement>(null);
  useEffect(() => () => { context.current = null; }, []);
  useEffect(() => {
    cache.current = null; setFolders([]); setError(false); setLoading(false); setOpen(false);
    setQuery(value); dirty.current = false; setActive(-1);
  }, [identity]);
  useEffect(() => { if (!dirty.current) setQuery(value); }, [value]);
  const options = ['', ...folders.filter(folder => folder.toLocaleLowerCase().includes(dirty.current ? query.toLocaleLowerCase() : ''))];
  useEffect(() => { list.current?.querySelector<HTMLElement>('[data-active="true"]')?.scrollIntoView?.({ block: 'nearest' }); }, [active]);

  async function load(force = false) {
    if (!workspacePath || (!force && cache.current)) return;
    const request = {};
    cache.current = request; setLoading(true); setError(false);
    const found = new Set<string>(), visited = new Set<string>(), pending = [workspacePath];
    let failed = false;
    while (pending.length && context.current === identity && cache.current === request) {
      const path = pending.pop()!;
      if (visited.has(path)) continue;
      visited.add(path);
      try {
        const entries = await readDirectory(path);
        if (context.current !== identity || cache.current !== request) return;
        for (const entry of entries) {
          if (entry.type !== 'folder') continue;
          const relative = relativePath(workspacePath, entry.id);
          if (!relative || relative.split('/').some(segment => !segment || segment.startsWith('.') || segment === 'node_modules')) continue;
          found.add(relative); pending.push(entry.id);
        }
      } catch { failed = true; }
    }
    if (context.current !== identity || cache.current !== request) return;
    setFolders([...found].sort((a, b) => a.localeCompare(b))); setError(failed); setLoading(false);
  }
  function choose(folder: string) { dirty.current = false; setQuery(folder); setOpen(false); setActive(-1); onChange(folder); }
  return <div className="q-daily-folder" onBlur={event => {
    if (event.relatedTarget instanceof Node && event.currentTarget.contains(event.relatedTarget)) return;
    if (dirty.current) choose(query); else setOpen(false);
  }}>
    <div className="q-input">
      <input className="q-input__field q-daily-folder__input" role="combobox" autoComplete="off"
        aria-label={t('settings.dailyNotes.folder')} aria-autocomplete="list" aria-haspopup="listbox"
        aria-expanded={open} aria-controls={open ? id : undefined}
        aria-activedescendant={open && active >= 0 ? id + '-' + active : undefined}
        value={query} placeholder={t('settings.dailyNotes.folderRoot')}
        onFocus={() => { setOpen(true); setActive(-1); void load(); }}
        onInput={event => { dirty.current = true; setQuery(event.currentTarget.value); setActive(-1); setOpen(true); }}
        onClick={() => { setOpen(true); void load(); }}
        onKeyDown={event => {
          if (event.key === 'Escape') {
            event.preventDefault(); event.stopPropagation(); dirty.current = false; setQuery(value); setOpen(false); setActive(-1);
          } else if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
            event.preventDefault(); setOpen(true); void load();
            setActive(current => event.key === 'ArrowDown' ? Math.min(current + 1, options.length - 1) : Math.max(current - 1, 0));
          } else if (event.key === 'Enter') {
            event.preventDefault(); choose(open && active >= 0 && options[active] !== undefined ? options[active] : query);
          }
        }} />
    </div>
    {open && <div className="q-daily-folder__popup">
      {loading && <p role="status">{t('settings.dailyNotes.folderLoading')}</p>}
      {error && <p role="alert">{t('settings.dailyNotes.folderError')} <button type="button" onMouseDown={event => event.preventDefault()} onClick={() => void load(true)}>{t('settings.dailyNotes.folderRetry')}</button></p>}
      <div id={id} role="listbox" aria-label={t('settings.dailyNotes.folder')} ref={list} className="q-daily-folder__options">
        {options.map((folder, index) => <button type="button" role="option" tabIndex={-1} id={id + '-' + index} key={folder}
          aria-selected={folder === value} data-active={index === active} className="q-daily-folder__option"
          onMouseDown={event => event.preventDefault()} onClick={() => choose(folder)}>{folder || t('settings.dailyNotes.folderRoot')}</button>)}
      </div>
    </div>}
  </div>;
}
