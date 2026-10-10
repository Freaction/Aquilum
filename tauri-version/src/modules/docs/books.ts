import { convertFileSrc } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { readFileStat } from '../documents/fileGateway';
import { absolutePath, fileName } from '../paths';
import { resolveVaultAbsolutePath } from './vaultAssets';
import { importIntoFiles, splitNameExt } from './vaultFiles';

const BOOK_EXTENSIONS = ['epub', 'mobi', 'azw3', 'fb2'] as const;

const MIME_BY_EXT: Record<string, string> = {
  epub: 'application/epub+zip',
  mobi: 'application/x-mobipocket-ebook',
  azw3: 'application/vnd.amazon.mobi8-ebook',
  fb2: 'application/x-fictionbook+xml',
};

async function importBookFile(
  workspacePath: string,
  sourcePath: string,
): Promise<{ relative: string; byteLength: number }> {
  const relative = await importIntoFiles(
    workspacePath,
    { kind: 'path', sourcePath },
    { fallbackName: 'book', fallbackExt: 'epub' },
  );
  const { byteLength } = await readFileStat(absolutePath(workspacePath, relative));
  return { relative, byteLength };
}

export async function pickAndImportBook(
  workspacePath: string,
): Promise<{ relative: string; byteLength: number } | null> {
  const selected = await open({
    multiple: false,
    filters: [{ name: 'Books', extensions: [...BOOK_EXTENSIONS] }],
  });
  if (typeof selected !== 'string' || !selected) return null;
  return importBookFile(workspacePath, selected);
}

export async function loadBookFile(
  workspacePath: string | null | undefined,
  bookFile: string,
): Promise<{ file: File; byteLength: number }> {
  const absolute = resolveVaultAbsolutePath(workspacePath, bookFile);
  const name = fileName(absolute);
  const { ext } = splitNameExt(name, 'epub');
  const url = convertFileSrc(absolute);

  const [{ byteLength }, blob] = await Promise.all([
    readFileStat(absolute),
    fetch(url).then((response) => {
      if (!response.ok) {
        throw new Error(`Не удалось загрузить книгу (${response.status})`);
      }
      return response.blob();
    }),
  ]);

  return {
    file: new File([blob], name, {
      type: MIME_BY_EXT[ext] ?? 'application/octet-stream',
    }),
    byteLength,
  };
}

export function hasBookFile(bookFile: string | undefined): boolean {
  return typeof bookFile === 'string' && bookFile.trim().length > 0;
}
