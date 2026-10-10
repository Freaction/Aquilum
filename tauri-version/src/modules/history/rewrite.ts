import { revertDocument, rewriteDocument } from '../documents/documentGateway';
import { PathQueue } from '../pathQueue';
import { readNoteVersion, type NoteVersion } from './index';
import { closeVersion } from './openedVersions';

export type Rewrite = (path: string, version: NoteVersion) => Promise<boolean>;

const running = new PathQueue<boolean>();

function once(path: string, task: () => Promise<boolean>): Promise<boolean> {
  return running.current(path) ?? running.run(path, task);
}

export async function applyFromHistory(
  rewrite: Rewrite,
  path: string,
  version: NoteVersion,
  tabId: string | null,
): Promise<boolean> {
  const done = await rewrite(path, version);
  if (done && tabId) closeVersion(tabId, version);
  return done;
}

export function restoreVersion(path: string, version: NoteVersion): Promise<boolean> {
  return once(path, async () => {
    const texts = await readNoteVersion(path, version.id);
    if (!texts) return false;
    return rewriteDocument(path, () => texts.text, { kind: 'restore', fromMs: version.atMs });
  });
}

export function revertVersion(path: string, version: NoteVersion): Promise<boolean> {
  return once(path, async () => {
    const texts = await readNoteVersion(path, version.id);
    if (!texts || texts.previous === null) return false;
    return revertDocument(path, texts.text, texts.previous, version.atMs);
  });
}
