import { open } from '@tauri-apps/plugin-dialog';
import { IMAGE_EXTENSIONS, mediaExtensionForMime } from './imageEmbeds';
import { importIntoFiles } from './vaultFiles';
import { resolveVaultAssetUrl } from './vaultAssets';
import {
  coverPatternIdFrom,
  DEFAULT_COVER_PATTERN,
  type CoverPatternId,
} from './coverPatterns';
import { clamp } from '../math';

export const DEFAULT_COVER_POSITION = '50%';

type PageCoverVisual =
  | { kind: 'pattern'; id: CoverPatternId }
  | { kind: 'image'; url: string };

export function resolvePageCoverVisual(
  workspacePath: string | null | undefined,
  pageCoverUrl: string | undefined,
): PageCoverVisual {
  const patternId = coverPatternIdFrom(pageCoverUrl);
  if (patternId) return { kind: 'pattern', id: patternId };
  const url = resolveVaultAssetUrl(workspacePath, pageCoverUrl, '');
  return url ? { kind: 'image', url } : { kind: 'pattern', id: DEFAULT_COVER_PATTERN };
}

export function parseCoverPositionY(value: string | undefined): number {
  if (!value) return 50;
  const trimmed = value.trim();
  const legacy = trimmed.match(/^(-?[\d.]+)%\s+(-?[\d.]+)%$/);
  if (legacy) return clamp(Number(legacy[2]), 0, 100);
  const single = trimmed.match(/^(-?[\d.]+)%$/);
  if (single) return clamp(Number(single[1]), 0, 100);
  return 50;
}

export function formatCoverPositionY(y: number): string {
  return `${clamp(Math.round(y), 0, 100)}%`;
}

async function importCoverFile(
  workspacePath: string,
  sourcePath: string,
): Promise<string> {
  return importIntoFiles(
    workspacePath,
    { kind: 'path', sourcePath },
    { fallbackName: 'cover', fallbackExt: 'jpg' },
  );
}

export async function importCoverBytes(
  workspacePath: string,
  file: File,
): Promise<string> {
  return importIntoFiles(
    workspacePath,
    { kind: 'bytes', bytes: new Uint8Array(await file.arrayBuffer()), name: file.name },
    { fallbackName: 'cover', fallbackExt: mediaExtensionForMime(file.type) },
  );
}

export async function pickAndImportCover(workspacePath: string): Promise<string | null> {
  const selected = await open({
    multiple: false,
    filters: [{ name: 'Images', extensions: IMAGE_EXTENSIONS }],
  });
  if (typeof selected !== 'string' || !selected) return null;
  return importCoverFile(workspacePath, selected);
}
