import type { ReactNode } from 'react';
import {
  BookOpen,
  CalendarDays,
  History,
  Keyboard,
  Monitor,
  Plug,
  Puzzle,
  Power,
  Search,
  Trash2,
  Type,
  FileText,
  Paperclip,
} from 'lucide';
import { Icon } from '../Common/Icon';
import { t } from '../../i18n';
import { GraphAnalysisIcon } from '../Icons/LinkIcons';
import type { SettingsSectionId } from './types';

export function settingsSections(): { id: SettingsSectionId; label: string }[] {
  return [
    { id: 'ui', label: t('settings.nav.ui') },
    { id: 'editor', label: t('settings.nav.editor') },
    { id: 'reader', label: t('settings.nav.reader') },
    { id: 'search', label: t('settings.nav.search') },
    { id: 'dailyNotes', label: t('settings.nav.dailyNotes') },
    { id: 'templates', label: t('settings.nav.templates') },
    { id: 'files', label: t('settings.nav.files') },
    { id: 'analysis', label: t('settings.nav.analysis') },
    { id: 'plugins', label: t('settings.nav.plugins') },
    { id: 'mcp', label: t('settings.nav.mcp') },
    { id: 'history', label: t('settings.nav.history') },
    { id: 'trash', label: t('settings.nav.trash') },
    { id: 'system', label: t('settings.nav.system') },
    { id: 'shortcuts', label: t('settings.nav.shortcuts') },
  ];
}

export const SETTINGS_SECTION_ICONS: Record<SettingsSectionId, ReactNode> = {
  ui: <Icon icon={Monitor} />,
  editor: <Icon icon={Type} />,
  reader: <Icon icon={BookOpen} />,
  search: <Icon icon={Search} />,
  dailyNotes: <Icon icon={CalendarDays} />,
  templates: <Icon icon={FileText} />,
  files: <Icon icon={Paperclip} />,
  analysis: <GraphAnalysisIcon />,
  plugins: <Icon icon={Puzzle} />,
  mcp: <Icon icon={Plug} />,
  history: <Icon icon={History} />,
  trash: <Icon icon={Trash2} />,
  system: <Icon icon={Power} />,
  shortcuts: <Icon icon={Keyboard} />,
};
