import { useEffect, useMemo, useRef, useState } from 'react';
import type { LinkDisposition } from '../../modules/links';
import { comparablePath } from '../../modules/paths';
import { useStableCallback } from '../../hooks/useStableCallback';
import type { VisibleFileRow } from './fileTreeModel';

const NOTHING_SELECTED: ReadonlySet<string> = new Set<string>();

function cleared(current: ReadonlySet<string>): ReadonlySet<string> {
  return current.size === 0 ? current : NOTHING_SELECTED;
}

export function useFileSelection(
  activeFile: string | null,
  rows: readonly VisibleFileRow[],
  onFileOpen: (path: string, options?: { disposition?: LinkDisposition }) => void,
) {
  const [selectedFiles, setSelectedFiles] = useState<ReadonlySet<string>>(NOTHING_SELECTED);
  const anchorRef = useRef<string | null>(null);

  useEffect(() => {
    anchorRef.current = activeFile;
    setSelectedFiles(cleared);
  }, [activeFile]);

  const selectRange = useStableCallback((path: string) => {
    const anchor = comparablePath(anchorRef.current);
    const from = rows.findIndex((row) => row.comparableId === anchor);
    const to = rows.findIndex((row) => row.comparableId === comparablePath(path));
    if (from < 0 || to < 0) {
      setSelectedFiles(new Set([path]));
      anchorRef.current = path;
      return;
    }
    const [start, end] = from < to ? [from, to] : [to, from];
    setSelectedFiles(new Set(rows.slice(start, end + 1).map((row) => row.item.id)));
  });

  const selectFile = useStableCallback((path: string, isMulti: boolean, isRange: boolean) => {
    if (isRange) {
      selectRange(path);
      return;
    }

    if (isMulti) {
      setSelectedFiles((current) => {
        const next = new Set(current);
        if (next.has(path)) next.delete(path);
        else next.add(path);
        return next;
      });
      anchorRef.current = path;
      return;
    }

    setSelectedFiles(cleared);
    anchorRef.current = path;
    onFileOpen(path);
  });

  const openInNewTab = useStableCallback((path: string) => {
    setSelectedFiles(cleared);
    anchorRef.current = path;
    onFileOpen(path, { disposition: 'new-tab' });
  });

  const targetsFor = useStableCallback((path: string) => (
    selectedFiles.size > 1 && selectedFiles.has(path) ? Array.from(selectedFiles) : [path]
  ));

  const focusRow = useStableCallback((path: string) => {
    if (selectedFiles.size > 1 && selectedFiles.has(path)) return;
    setSelectedFiles(new Set([path]));
    anchorRef.current = path;
  });

  const clearSelection = useStableCallback(() => {
    setSelectedFiles(cleared);
  });

  const rowActions = useMemo(
    () => ({ selectFile, focusRow, openInNewTab }),
    [focusRow, openInNewTab, selectFile],
  );

  return { selectedFiles, rowActions, targetsFor, clearSelection };
}
