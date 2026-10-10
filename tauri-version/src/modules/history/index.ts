import { invoke } from '@tauri-apps/api/core';
import { formatDateTime, t } from '../../i18n';

type VersionSource = 'me' | 'agent' | 'external' | 'links' | 'start' | 'restore' | 'revert';

export interface NoteVersion {
  id: string;
  atMs: number;
  device: string;
  source: VersionSource;
  fromMs: number | null;
  name: string | null;
  isCurrent: boolean;
}

interface HistoryPage {
  total: number;
  versions: NoteVersion[];
  editing?: VersionSource | null;
}

export interface VersionTexts {
  text: string;
  previous: string | null;
}

export interface NoteHistoryChanged {
  path: string;
}

export const NOTE_HISTORY_EVENT = 'note-history-changed';

export function listNoteHistory(path: string, offset: number, limit: number): Promise<HistoryPage> {
  return invoke<HistoryPage>('note_history', { path, offset, limit });
}

export function readNoteVersion(path: string, version: string | null): Promise<VersionTexts | null> {
  return invoke<VersionTexts | null>('read_note_version', { path, version });
}

export function nameNoteVersion(path: string, version: string | null, name: string): Promise<string | null> {
  return invoke<string | null>('name_note_version', { path, version, name });
}

export function cleanupHistory(workspacePath: string, retentionDays: number): Promise<number> {
  return invoke<number>('cleanup_history', { workspacePath, retentionDays });
}

function versionSourceLabel(source: VersionSource): string {
  return t(`history.sources.${source}`);
}

export function versionLabel(version: NoteVersion): string {
  return version.name ?? versionSourceLabel(version.source);
}

export function versionTitle(version: NoteVersion): string {
  if (version.fromMs === null) return versionSourceLabel(version.source);
  const key = version.source === 'revert' ? 'history.revertedFrom' : 'history.restoredFrom';
  return t(key, { date: formatDateTime(version.fromMs) });
}

export function historyRetentionOptions() {
  return [
    { value: '0', label: t('settings.history.forever') },
    { value: '30', label: t('settings.history.days', { count: 30 }) },
    { value: '90', label: t('settings.history.days', { count: 90 }) },
    { value: '365', label: t('settings.history.year') },
  ];
}

export { groupByDay } from './days';
export {
  closeVersion,
  keepVersionsOf,
  openVersion,
  rememberScroll,
  useOpenedVersion,
  type OpenedVersion,
} from './openedVersions';
export { applyFromHistory, restoreVersion, revertVersion, type Rewrite } from './rewrite';
export { useNoteHistory } from './useNoteHistory';
