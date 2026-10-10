import { invoke } from '@tauri-apps/api/core';
import { formatNumber, plural, t } from '../../i18n';

export interface TrashState {
  path: string;
  count: number;
  bytes: number;
}

export interface TrashedDeletion {
  id: string;
  original: string;
  files: number;
  deletedAtMs: number;
}

export interface TrashPage {
  total: number;
  deletions: TrashedDeletion[];
}

export function trashRetentionOptions() {
  return [
    { value: '0', label: t('settings.trash.never') },
    { value: '7', label: t('settings.trash.days', { count: 7 }) },
    { value: '30', label: t('settings.trash.days', { count: 30 }) },
    { value: '90', label: t('settings.trash.days', { count: 90 }) },
    { value: '365', label: t('settings.trash.year') },
  ];
}

export function getTrashState(workspacePath: string): Promise<TrashState> {
  return invoke<TrashState>('get_trash_state', { workspacePath });
}

export function listTrash(workspacePath: string, offset: number, limit: number): Promise<TrashPage> {
  return invoke<TrashPage>('list_trash', { workspacePath, offset, limit });
}

export function restoreDeletion(workspacePath: string, id: string): Promise<void> {
  return invoke<void>('restore_deletion', { workspacePath, id });
}

export function cleanupTrash(workspacePath: string, retentionDays: number): Promise<number> {
  return invoke<number>('cleanup_trash', { workspacePath, retentionDays });
}

export function openTrash(workspacePath: string): Promise<void> {
  return invoke<void>('open_trash', { workspacePath });
}

export function formatTrashSize(bytes: number): string {
  if (bytes < 1024) return t('settings.trash.bytes', { value: formatNumber(bytes) });
  if (bytes < 1024 * 1024) {
    return t('settings.trash.kilobytes', { value: formatNumber(Math.round(bytes / 1024)) });
  }
  return t('settings.trash.megabytes', { value: formatNumber(bytes / (1024 * 1024), 1) });
}

export function formatTrashCount(count: number): string {
  return plural('settings.trash.files', count);
}
