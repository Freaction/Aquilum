import { useCallback, useEffect, useRef, useState } from 'react';
import { useTauriEvent } from '../../hooks/useTauriEvent';
import { samePath } from '../paths';
import {
  listNoteHistory,
  NOTE_HISTORY_EVENT,
  type NoteHistoryChanged,
  type NoteVersion,
} from './index';

const PAGE_SIZE = 50;

interface LoadedHistory {
  path: string;
  total: number;
  versions: NoteVersion[];
}

export function useNoteHistory(path: string | null, enabled: boolean) {
  const [loaded, setLoaded] = useState<LoadedHistory | null>(null);
  const [revision, setRevision] = useState(0);
  const request = useRef(0);

  useEffect(() => {
    if (!enabled || !path) return;
    const token = ++request.current;
    void listNoteHistory(path, 0, PAGE_SIZE).then((page) => {
      if (token === request.current) {
        setLoaded({ path, total: page.total, versions: page.versions });
      }
    }).catch((error) => console.error('Failed to read the note history', error));
  }, [enabled, path, revision]);

  useTauriEvent<NoteHistoryChanged>(NOTE_HISTORY_EVENT, (changed) => {
    if (path && samePath(changed.path, path)) setRevision((current) => current + 1);
  }, enabled && path !== null);

  const current = loaded && path && loaded.path === path ? loaded : null;

  const showMore = useCallback(() => {
    if (!current) return;
    const token = ++request.current;
    void listNoteHistory(current.path, current.versions.length, PAGE_SIZE).then((page) => {
      if (token !== request.current) return;
      setLoaded({
        path: current.path,
        total: page.total,
        versions: [...current.versions, ...page.versions],
      });
    }).catch((error) => console.error('Failed to read the note history', error));
  }, [current]);

  return { history: current, showMore };
}
