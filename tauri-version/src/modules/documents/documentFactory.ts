import { absolutePath, childPath } from '../paths';
import { createAtFreeName, createFile, isMarkdownPath } from './fileGateway';

const UNTITLED_NOTE = 'Без названия';

export async function createUniqueFile(
  workspacePath: string,
  preferredTitle?: string,
  content = '',
): Promise<string | null> {
  const baseName = sanitizeFileName(preferredTitle) || UNTITLED_NOTE;
  try {
    return await createAtFreeName(
      (attempt) => childPath(workspacePath, `${attempt === 0 ? baseName : `${baseName} ${attempt}`}.md`),
      (path) => createFile(path, content),
    );
  } catch (error) {
    console.error('Failed to create document', error);
    return null;
  }
}

export function linkedFilePath(workspacePath: string, target: string): string | null {
  const normalized = target.trim().replace(/\\/g, '/');
  const withoutExtension = isMarkdownPath(normalized) ? normalized.slice(0, -3) : normalized;
  const segments = withoutExtension.split('/');
  if (segments.length === 0 || segments.some((segment) => (
    !segment || segment !== segment.trim() || sanitizeFileName(segment) !== segment
  ))) return null;
  return absolutePath(workspacePath, `${segments.join('/')}.md`);
}

export function sanitizeFileName(value?: string): string {
  if (!value) return '';
  const sanitized = value
    .replace(/[<>:"/\\|?*\u0000-\u001f]/g, ' ')
    .replace(/\s+/g, ' ')
    .trim()
    .replace(/[. ]+$/g, '')
    .slice(0, 120)
    .trim();
  return /^(con|prn|aux|nul|com[1-9]|lpt[1-9])$/i.test(sanitized) ? `${sanitized} note` : sanitized;
}
