import { useState } from 'react';
import { Button } from '../../components/Common/Button';
import { Dropdown } from '../../components/Common/Dropdown';
import { Input } from '../../components/Common/Input';
import { Switch } from '../../components/Common/Switch';
import { Row } from '../../components/Settings/Row';
import { t } from '../../i18n';
import type { PluginSettingsProps } from '../registry';
import { usePluginVaultData, type Filter, type VaultPluginData } from '../vaultData';
import { useExplorerOverview } from './useExplorerOverview';
import './Settings.css';

type List = 'hide' | 'pin';
const label = (key: string) => t(`plugins.explorerFilters.${key}`);

function FilterEditor({ filter, pending, onSave, onCancel }: { filter: Filter; pending: boolean; onSave: (filter: Filter) => void; onCancel: () => void }) {
  const [draft, setDraft] = useState(filter);
  const valid = draft.pattern.length > 0 && (draft.kind !== 'path' || draft.patternType !== 'strict' || (!/[\\:\p{Cc}]/u.test(draft.pattern) && draft.pattern.split('/').every(part => part !== '' && part !== '.' && part !== '..')));
  return <div className="q-explorer-filter-editor">
    {(['name', 'pattern'] as const).map(field => <Row key={field} label={label(field === 'name' ? 'filterName' : field)}><Input ariaLabel={label(field === 'name' ? 'filterName' : field)} value={draft[field]} disabled={pending} onChange={value => setDraft({ ...draft, [field]: value })} /></Row>)}
    {([['kind', ['path', 'tag']], ['target', ['files', 'folders', 'both']], ['patternType', ['strict', 'wildcard', 'regex']]] as const).map(([field, values]) => <Row key={field} label={label(field)}><Dropdown ariaLabel={label(field)} value={draft[field]} disabled={pending} options={values.map(value => ({ value, label: label(value) }))} onChange={value => setDraft({ ...draft, [field]: value })} /></Row>)}
    <Row label={label('active')}><Switch label={label('active')} checked={draft.active} disabled={pending} onChange={active => setDraft({ ...draft, active })} /></Row>
    {!valid && <p className="q-explorer-filter-error" role="status">{label('invalidPattern')}</p>}
    <div className="q-explorer-filter-actions"><Button disabled={pending || !valid} onClick={() => onSave(draft)}>{label('save')}</Button><Button disabled={pending} onClick={onCancel}>{label('cancel')}</Button></div>
  </div>;
}

function FilterLists({ config, workspacePath }: PluginSettingsProps) {
  const { data, update } = usePluginVaultData(workspacePath);
  const { overview, error: overviewError } = useExplorerOverview(workspacePath, data, config.plugins);
  const [editing, setEditing] = useState<{ list: List; index: number; filter: Filter } | null>(null);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState(false);
  const disabled = !workspacePath || !data || pending;
  const save = async (mutate: (data: VaultPluginData) => void, close = false) => {
    setPending(true); setError(false);
    try { await update(mutate); if (close) setEditing(null); }
    catch { setError(true); }
    finally { setPending(false); }
  };
  return <div className="q-explorer-filter-settings">
    {!workspacePath && <p role="status">{label('noWorkspace')}</p>}
    {workspacePath && !data && <p role="status">{label('loading')}</p>}
    {error && <p className="q-explorer-filter-error" role="alert">{label('saveError')}</p>}
    {overviewError && <p className="q-explorer-filter-error" role="alert">{label('overviewError')}: {overviewError}</p>}
    {(['hide', 'pin'] as const).map(list => <section key={list} aria-label={label(`${list}List`)}>
      <h4>{label(`${list}List`)}</h4>
      {data?.filters[list].map((filter, index) => <div className="q-explorer-filter" key={index}>
        <Row label={filter.name || filter.pattern}><Switch label={filter.name || filter.pattern} checked={filter.active} disabled={disabled || !!editing} onChange={active => { void save(next => { next.filters[list][index].active = active; }); }} /></Row>
        <p className="q-explorer-filter-pattern">{filter.pattern}</p>
        {overview.errors.filter(item => item.list === list && item.index === index).map((item, i) => <p key={i} className="q-explorer-filter-error" role="alert">{item.message}</p>)}
        <div className="q-explorer-filter-actions"><Button disabled={disabled || !!editing} onClick={() => setEditing({ list, index, filter })}>{label('edit')}</Button><Button disabled={disabled || !!editing} onClick={() => { void save(next => { next.filters[list].splice(index, 1); }); }}>{label('remove')}</Button></div>
      </div>)}
      <Button disabled={disabled || !!editing} onClick={() => setEditing({ list, index: -1, filter: { name: '', active: true, kind: 'path', target: 'both', pattern: '', patternType: 'wildcard' } })}>{label(list === 'hide' ? 'addHide' : 'addPin')}</Button>
      {editing?.list === list && <FilterEditor key={`${list}:${editing.index}`} filter={editing.filter} pending={pending} onCancel={() => setEditing(null)} onSave={filter => { void save(next => {
        if (editing.index === -1) next.filters[list].push(filter);
        else {
          if (JSON.stringify(next.filters[list][editing.index]) !== JSON.stringify(editing.filter)) throw new Error('Filter changed during editing');
          next.filters[list][editing.index] = filter;
        }
      }, true); }} />}
    </section>)}
  </div>;
}

export function ExplorerFiltersSettings(props: PluginSettingsProps) {
  return <FilterLists key={props.workspacePath} {...props} />;
}
