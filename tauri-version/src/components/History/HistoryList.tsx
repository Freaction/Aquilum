import { useState } from 'react';
import { formatDateTime, formatTime, t } from '../../i18n';
import { samePath } from '../../modules/paths';
import {
  applyFromHistory,
  closeVersion,
  groupByDay,
  nameNoteVersion,
  openVersion,
  restoreVersion,
  revertVersion,
  useNoteHistory,
  useOpenedVersion,
  versionLabel,
  versionTitle,
  type NoteVersion,
  type Rewrite,
} from '../../modules/history';
import type { MenuItem } from '../Common/Menu';
import { TextButton } from '../Common/TextButton';
import { renameItem } from '../Layout/fileActionItems';
import { HistoryRow } from './HistoryRow';
import './HistoryList.css';

interface HistoryListProps {
  path: string;
  tabId: string | null;
  isOpen: boolean;
}

export function HistoryList({ path, tabId, isOpen }: HistoryListProps) {
  const { history, showMore } = useNoteHistory(path, isOpen);
  const opened = useOpenedVersion(tabId);
  const [renaming, setRenaming] = useState<{ path: string; key: string } | null>(null);
  if (!history) return null;
  const selected = opened && samePath(opened.path, path) ? opened.version.id : null;
  const renamingKey = renaming?.path === path ? renaming.key : null;

  const open = (version: NoteVersion) => {
    if (tabId) openVersion(tabId, path, version);
  };
  const openCurrent = () => {
    if (tabId) closeVersion(tabId);
  };
  const rename = (version: string, name: string | null, label: string) => (typed: string) => {
    setRenaming(null);
    const next = typed.trim();
    if (next === label || next === (name ?? '')) return;
    void nameNoteVersion(path, version, next).catch((error) => {
      console.error('Failed to name the note version', error);
    });
  };
  const renameState = (key: string) => ({
    renaming: renamingKey === key,
    onCancelRename: () => setRenaming(null),
  });
  const startRename = (key: string) => renameItem(() => setRenaming({ path, key }));
  const change = (rewrite: Rewrite, version: NoteVersion) => {
    void applyFromHistory(rewrite, path, version, tabId).catch((error) => {
      console.error('Failed to change the note from its history', error);
    });
  };
  const loaded = history.versions;
  const oldest = loaded.length === history.total ? loaded[loaded.length - 1]?.id : null;
  const versionItems = (version: NoteVersion): MenuItem[] => [
    {
      id: 'restore',
      label: t('history.restore'),
      disabled: version.isCurrent,
      onSelect: () => change(restoreVersion, version),
    },
    ...(version.id === oldest
      ? []
      : [{ id: 'revert', label: t('history.revert'), onSelect: () => change(revertVersion, version) }]),
    startRename(version.id),
  ];

  return (
    <>
      <HistoryRow
        label={t('history.current')}
        selected={selected === null}
        onOpen={openCurrent}
      />
      {history.total === 0 && <p className="q-panel-empty">{t('history.empty')}</p>}
      {groupByDay(history.versions, Date.now()).map((day) => (
        <section key={day.key} className="q-history-list__day" aria-label={day.label}>
          <h3 className="q-history-list__day-title">{day.label}</h3>
          {day.versions.map((version) => (
            <HistoryRow
              key={version.id}
              label={versionLabel(version)}
              counter={formatTime(version.atMs)}
              title={`${versionTitle(version)} · ${formatDateTime(version.atMs)}`}
              selected={version.id === selected}
              menuItems={versionItems(version)}
              onOpen={() => open(version)}
              onRename={rename(version.id, version.name, versionLabel(version))}
              {...renameState(version.id)}
            />
          ))}
        </section>
      ))}
      {history.versions.length < history.total && (
        <TextButton className="q-history-list__more" onClick={showMore}>
          {t('history.more')}
        </TextButton>
      )}
    </>
  );
}
