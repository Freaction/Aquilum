import { applyPlaceholders } from './placeholders';
import { bookStarterTemplate, noteStarterTemplate } from './starter';
import {
  createFile,
  ensureDirectory,
  isFileCommandError,
  isMarkdownPath,
  readDirectory,
  readFileSnapshot,
} from '../documents/fileGateway';
import { childPath } from '../paths';

export interface NoteTemplate { name: string; path: string; relativePath: string }

const DEFAULT_TEMPLATES_FOLDER = 'Templates';

export function templatesFolderName(folder: string) {
  return folder.trim() || DEFAULT_TEMPLATES_FOLDER;
}

function templatesPath(workspacePath: string, folder: string) {
  const value = folder.trim();
  if (/^(?:[a-z]:[\\/]|\\\\|\/)/i.test(value)) return value.replace(/[\\/]+$/g, '');
  const normalized = value.replace(/^[\\/]+|[\\/]+$/g, '');
  return childPath(workspacePath, normalized || DEFAULT_TEMPLATES_FOLDER);
}

export async function createStarterTemplates(workspacePath: string, folder: string) {
  const root = templatesPath(workspacePath, folder);
  await ensureDirectory(root);
  await Promise.all([
    createIfMissing(childPath(root, 'Note.md'), noteStarterTemplate),
    createIfMissing(childPath(root, 'Book.md'), bookStarterTemplate),
  ]);
}

export async function templatesDirectoryExists(workspacePath: string, folder: string) {
  try {
    await readDirectory(templatesPath(workspacePath, folder));
    return true;
  } catch {
    return false;
  }
}

async function createIfMissing(path: string, content: string) {
  try { await createFile(path, content); }
  catch (error) { if (!isFileCommandError(error, 'already_exists')) throw error; }
}

export async function listTemplates(workspacePath: string, folder: string): Promise<NoteTemplate[]> {
  const root = templatesPath(workspacePath, folder);
  const results: NoteTemplate[] = [];
  async function visit(path: string, prefix = '') {
    const items = await readDirectory(path);
    await Promise.all(items.map(async (item) => {
      const relativePath = prefix ? `${prefix}/${item.name}` : item.name;
      if (item.type === 'folder') await visit(item.id, relativePath);
      else if (isMarkdownPath(item.id)) {
        results.push({ name: item.name, path: item.id, relativePath });
      }
    }));
  }
  try { await visit(root); } catch { return []; }
  return results.sort((a, b) => a.relativePath.localeCompare(b.relativePath));
}

export async function readTemplate(template: NoteTemplate) {
  const { content } = await readFileSnapshot(template.path);
  return applyPlaceholders(content, new Date());
}
