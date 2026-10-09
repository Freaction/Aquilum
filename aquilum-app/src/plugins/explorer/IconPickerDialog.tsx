import { useEffect, useRef, useState } from 'react';
import { Button } from '../../components/Common/Button';
import { Dialog, DialogFooter } from '../../components/Common/Dialog';
import { Input } from '../../components/Common/Input';
import { Icon } from '../../components/Common/Icon';
import type { CodeMirrorFieldRef } from '../../components/Common/CodeMirrorField';
import { t } from '../../i18n';
import { ICON_NAMES, iconByName } from './icons';
import { usePickerFocus } from './usePickerFocus';
import './Pickers.css';

interface IconPickerDialogProps {
  open: boolean;
  value?: string;
  recentIcons: readonly string[];
  onSelect: (name: string | null) => void;
  onClose: () => void;
  pending?: boolean;
  error?: string | null;
}

export function IconPickerDialog({ open, value, recentIcons, onSelect, onClose, pending = false, error }: IconPickerDialogProps) {
  const bodyRef = usePickerFocus(open);
  const [query, setQuery] = useState('');
  const searchRef = useRef<CodeMirrorFieldRef>(null);
  useEffect(() => { if (open) setQuery(''); }, [open]);
  const matches = ICON_NAMES.filter(name => name.includes(query.trim().toLowerCase())).slice(0, 200);
  const recent = [...new Set(recentIcons)].filter(name => iconByName(name)).slice(0, 5);
  const option = (name: string) => (
    <Button key={name} variant="ghost" className="q-icon-picker__option" title={name} aria-label={name} aria-pressed={value === name} disabled={pending} onClick={() => onSelect(name)}>
      <Icon icon={iconByName(name)!} size={24} />
    </Button>
  );
  return (
    <Dialog open={open} title={t('plugins.fileIcons.pickerTitle')} closeLabel={t('plugins.fileIcons.close')} onClose={onClose} initialFocus={() => searchRef.current?.focus()} className="q-icon-picker">
      <div ref={bodyRef} className="q-picker__body" aria-busy={pending}>
        <Input ref={searchRef} value={query} onChange={setQuery} ariaLabel={t('plugins.fileIcons.search')} placeholder={t('plugins.fileIcons.search')} fullWidth />
        {recent.length > 0 && <div className="q-icon-picker__recent" role="group" aria-label={t('plugins.fileIcons.recent')}><p className="q-icon-picker__label">{t('plugins.fileIcons.recent')}</p><div className="q-icon-picker__grid">{recent.map(option)}</div></div>}
        <div className="q-icon-picker__results q-icon-picker__grid">{matches.map(option)}</div>
        {matches.length === 0 && <p role="status">{t('plugins.fileIcons.empty')}</p>}
        {error && <p className="q-picker__error" role="alert">{error}</p>}
      </div>
      <DialogFooter><Button variant="ghost" className="q-icon-picker__remove" disabled={pending} onClick={() => onSelect(null)}>{t('plugins.fileIcons.remove')}</Button></DialogFooter>
    </Dialog>
  );
}
