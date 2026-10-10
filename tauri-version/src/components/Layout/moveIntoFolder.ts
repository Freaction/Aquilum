import { isRenameConflict, moveWorkspaceEntry } from '../../modules/documents/renameWorkspaceFile';
import { childPath, claimNumberedPath, fileName } from '../../modules/paths';

export async function moveIntoFolder(sourceId: string, targetFolderId: string): Promise<string | null> {
  try {
    return await claimNumberedPath(
      childPath(targetFolderId, fileName(sourceId)),
      (candidate) => moveWorkspaceEntry(sourceId, candidate),
      isRenameConflict,
    );
  } catch (error) {
    console.error('Failed to move file:', error);
    return null;
  }
}
