import { createFile, isFileCommandError } from '../modules/documents/fileGateway';
import { linkedFilePath } from '../modules/documents/documentFactory';
import { emptyTabPath, GRAPH_TAB_PATH, type SessionTab } from '../modules/ui-state';

export function createEmptySessionTab(): SessionTab {
  const tabId = crypto.randomUUID();
  return {
    tabId,
    documentId: null,
    kind: 'empty',
    path: emptyTabPath(tabId),
  };
}

export function createGraphSessionTab(): SessionTab {
  return {
    tabId: crypto.randomUUID(),
    documentId: null,
    kind: 'graph',
    path: GRAPH_TAB_PATH,
  };
}

export async function createLinkedFile(
  workspacePath: string,
  target: string,
): Promise<string | null> {
  const path = linkedFilePath(workspacePath, target);
  if (!path) return null;
  try {
    await createFile(path);
    return path;
  } catch (error) {
    if (isFileCommandError(error, 'already_exists')) return path;
    console.error('Failed to create linked file', error);
    return null;
  }
}
