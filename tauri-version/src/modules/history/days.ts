import { formatDay, t } from '../../i18n';
import type { NoteVersion } from './index';

interface DayGroup {
  key: string;
  label: string;
  versions: NoteVersion[];
}

function dayKey(epochMs: number): string {
  const date = new Date(epochMs);
  return `${date.getFullYear()}-${date.getMonth()}-${date.getDate()}`;
}

function dayLabel(epochMs: number, nowMs: number): string {
  const key = dayKey(epochMs);
  if (key === dayKey(nowMs)) return t('history.today');
  const yesterday = new Date(nowMs);
  yesterday.setDate(yesterday.getDate() - 1);
  if (key === dayKey(yesterday.getTime())) return t('history.yesterday');
  return formatDay(epochMs, new Date(epochMs).getFullYear() !== new Date(nowMs).getFullYear());
}

export function groupByDay(versions: NoteVersion[], nowMs: number): DayGroup[] {
  const groups: DayGroup[] = [];
  for (const version of versions) {
    const key = dayKey(version.atMs);
    const last = groups[groups.length - 1];
    if (last?.key === key) {
      last.versions.push(version);
    } else {
      groups.push({ key, label: dayLabel(version.atMs, nowMs), versions: [version] });
    }
  }
  return groups;
}
