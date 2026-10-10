import { isFileCommandError, renameFile } from './fileGateway';
import { fileStem, siblingPath } from '../paths';
import { sanitizeFileName } from './documentFactory';

export async function renameWorkspaceFile(oldPath: string, newStem: string): Promise<string | null> {
  const stem = sanitizeFileName(newStem);
  if (!stem || stem === fileStem(oldPath)) return null;
  const newPath = siblingPath(oldPath, `${stem}.md`);
  await renameFile(oldPath, newPath);
  return newPath;
}

export async function moveWorkspaceEntry(from: string, to: string): Promise<void> {
  await renameFile(from, to);
}

export function isRenameConflict(error: unknown): boolean {
  return isFileCommandError(error, 'already_exists');
}
