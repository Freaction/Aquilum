import { useMemo, useRef, useState } from 'react';
import { copyFile, isFileCommandError, isMarkdownPath, trashFile } from '../../modules/documents/fileGateway';
import { renameFolder } from '../../modules/documents/workspaceFolder';
import {
  isRenameConflict,
  renameWorkspaceFile,
} from '../../modules/documents/renameWorkspaceFile';
import { claimNumberedPath, fileStem } from '../../modules/paths';
import { useStableCallback } from '../../hooks/useStableCallback';

interface FileTreeActionsInput {
  workspacePath: string | null;
  targetsFor: (path: string) => string[];
  clearSelection?: () => void;
  onPatchFileInTree?: (path: string, patch: { id?: string; name?: string }) => void;
  onPathMoved?: (from: string, to: string) => void;
}

export function useFileTreeActions({
  workspacePath,
  targetsFor,
  clearSelection,
  onPatchFileInTree,
  onPathMoved,
}: FileTreeActionsInput) {
  const [renamingPath, setRenamingPath] = useState<string | null>(null);
  const [deleteTargets, setDeleteTargets] = useState<string[]>([]);
  const [isDeleting, setIsDeleting] = useState(false);
  const actionPendingRef = useRef(false);

  const startRename = useStableCallback((path: string) => {
    setRenamingPath(path);
  });

  const cancelRename = useStableCallback(() => {
    setRenamingPath(null);
  });

  const commitRename = useStableCallback(async (path: string, nextName: string) => {
    const current = fileStem(path);
    const trimmed = nextName.trim();
    setRenamingPath(null);
    if (!trimmed || trimmed === current) return;

    onPatchFileInTree?.(path, { name: trimmed });
    try {
      const newPath = isMarkdownPath(path)
        ? await renameWorkspaceFile(path, trimmed)
        : await renameFolder(path, trimmed);
      if (newPath) onPathMoved?.(path, newPath);
      else onPatchFileInTree?.(path, { name: current });
    } catch (error) {
      onPatchFileInTree?.(path, { name: current });
      if (isRenameConflict(error)) {
        console.warn(`File ${trimmed} already exists!`);
      } else {
        console.error('Rename failed', error);
      }
    }
  });

  const requestDelete = useStableCallback((path: string) => {
    setDeleteTargets(targetsFor(path));
  });

  const dismissDelete = useStableCallback(() => {
    setDeleteTargets([]);
  });

  const confirmDelete = useStableCallback(async () => {
    if (actionPendingRef.current || deleteTargets.length === 0) return;
    actionPendingRef.current = true;
    setIsDeleting(true);
    try {
      if (workspacePath) {
        for (const path of deleteTargets) {
          try {
            await trashFile(workspacePath, path);
          } catch (error) {
            console.error(`Delete failed for ${path}`, error);
          }
        }
      }
      clearSelection?.();
      setDeleteTargets([]);
    } finally {
      actionPendingRef.current = false;
      setIsDeleting(false);
    }
  });

  const duplicateFile = useStableCallback(async (path: string) => {
    if (actionPendingRef.current) return;
    actionPendingRef.current = true;
    try {
      for (const source of targetsFor(path)) {
        try {
          await claimNumberedPath(
            source,
            (candidate) => copyFile(source, candidate),
            (error) => isFileCommandError(error, 'already_exists'),
            1,
          );
        } catch (error) {
          console.error(`Duplicate failed for ${source}`, error);
        }
      }
    } finally {
      actionPendingRef.current = false;
    }
  });

  const rowActions = useMemo(() => ({
    startRename,
    commitRename,
    cancelRename,
    requestDelete,
    duplicateFile,
  }), [cancelRename, commitRename, duplicateFile, requestDelete, startRename]);

  return { renamingPath, deleteTargets, isDeleting, rowActions, confirmDelete, dismissDelete };
}
