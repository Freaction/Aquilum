import { siblingPath } from '../paths';
import { moveWorkspaceEntry } from './renameWorkspaceFile';

export async function renameFolder(folderPath: string, name: string): Promise<string> {
  const newPath = siblingPath(folderPath, name);
  await moveWorkspaceEntry(folderPath, newPath);
  return newPath;
}
