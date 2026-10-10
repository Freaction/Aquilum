import {
  copyFile,
  createAtFreeName,
  createBinaryFile,
  ensureDirectory,
} from '../documents/fileGateway';
import { sanitizeFileName } from '../documents/documentFactory';
import { absolutePath, fileName } from '../paths';

const DEFAULT_FILES_FOLDER = 'Files';

let filesFolder = DEFAULT_FILES_FOLDER;

export function normalizeFilesFolder(folder: string): string {
  const segments = folder.split(/[\\/]+/)
    .map((segment) => segment.trim())
    .filter((segment) => segment && segment !== '.' && segment !== '..' && !segment.includes(':'));
  return segments.join('/') || DEFAULT_FILES_FOLDER;
}

export function setFilesFolder(folder: string): void {
  filesFolder = normalizeFilesFolder(folder);
}

export function splitNameExt(
  name: string,
  fallbackExt: string,
): { stem: string; ext: string } {
  const dot = name.lastIndexOf('.');
  if (dot <= 0) return { stem: name, ext: fallbackExt };
  return {
    stem: name.slice(0, dot),
    ext: name.slice(dot + 1).toLowerCase() || fallbackExt,
  };
}

type FilesImportSource =
  | { kind: 'path'; sourcePath: string }
  | { kind: 'bytes'; bytes: Uint8Array; name: string };

type FilesImportOptions = {
  fallbackName: string;
  fallbackExt: string;
};

export async function importIntoFiles(
  workspacePath: string,
  source: FilesImportSource,
  { fallbackName, fallbackExt }: FilesImportOptions,
): Promise<string> {
  const folder = filesFolder;
  await ensureDirectory(absolutePath(workspacePath, folder));

  const original = source.kind === 'path' ? fileName(source.sourcePath) : source.name;
  const { stem, ext } = splitNameExt(original, fallbackExt);
  const baseName = sanitizeFileName(stem) || fallbackName;

  const relative = await createAtFreeName(
    (attempt) => `${folder}/${attempt === 0 ? baseName : `${baseName} ${attempt}`}.${ext}`,
    (candidate) => {
      const dest = absolutePath(workspacePath, candidate);
      return source.kind === 'path' ? copyFile(source.sourcePath, dest) : createBinaryFile(dest, source.bytes);
    },
  );
  if (!relative) throw new Error(`No free file name for ${baseName}.${ext}`);
  return relative;
}
