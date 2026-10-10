import { clamp } from '../math';

export type ImageAlign = 'left' | 'center' | 'right';

export type ImageCrop = {
  left: number;
  top: number;
  right: number;
  bottom: number;
};

export type MediaKind = 'image' | 'video';

export type ImageEmbed = {
  src: string;
  width: number | null;
  align: ImageAlign;
  crop: ImageCrop | null;
  kind: MediaKind;
  wiki: boolean;
};

type ImageEmbedInput =
  Omit<ImageEmbed, 'kind' | 'wiki'> & { kind?: MediaKind; wiki?: boolean };

export const DEFAULT_IMAGE_ALIGN: ImageAlign = 'center';
const IMAGE_ALIGN_ORDER: readonly ImageAlign[] = ['left', 'center', 'right'];
const EXTENSION_BY_MIME: Record<string, string> = {
  'image/png': 'png',
  'image/jpeg': 'jpg',
  'image/webp': 'webp',
  'image/gif': 'gif',
  'image/avif': 'avif',
  'image/bmp': 'bmp',
  'image/svg+xml': 'svg',
};

export const IMAGE_EXTENSIONS = [...new Set([...Object.values(EXTENSION_BY_MIME), 'jpeg'])];
const VIDEO_EXTENSIONS = ['mp4', 'webm', 'ogv', 'ogg', 'mov', 'm4v', 'mkv'];

function extensionOf(src: string): string {
  const withoutQuery = src.split(/[?#]/)[0] ?? '';
  const dot = withoutQuery.lastIndexOf('.');
  return dot < 0 ? '' : withoutQuery.slice(dot + 1).toLowerCase();
}

function mediaKindOf(src: string): MediaKind | null {
  const extension = extensionOf(src);
  if (VIDEO_EXTENSIONS.includes(extension)) return 'video';
  if (IMAGE_EXTENSIONS.includes(extension)) return 'image';
  return null;
}

export function isImageSource(src: string): boolean {
  return mediaKindOf(src) === 'image';
}

const EXTENSION_BY_VIDEO_MIME: Record<string, string> = {
  'video/mp4': 'mp4',
  'video/webm': 'webm',
  'video/ogg': 'ogv',
  'video/quicktime': 'mov',
  'video/x-m4v': 'm4v',
  'video/x-matroska': 'mkv',
};

export function mediaKindForMime(mime: string): MediaKind | null {
  if (mime.startsWith('image/')) return 'image';
  if (mime.startsWith('video/')) return 'video';
  return null;
}

export function mediaExtensionForMime(mime: string): string {
  if (mime.startsWith('video/')) return EXTENSION_BY_VIDEO_MIME[mime] ?? 'mp4';
  return EXTENSION_BY_MIME[mime] ?? 'png';
}

const IMAGE_LINE = /^!\[([^\]]*)\]\(\s*(?:<([^<>]+)>|([^()]+?))\s*\)$/;
const WIKI_MEDIA_LINE = /^!\[\[([^[\]]+)\]\]$/;
const ALIGNS = new Set<string>(IMAGE_ALIGN_ORDER);

export function roundCropPercent(value: number): number {
  return Math.round(value * 10) / 10;
}

function clampPercent(value: number): number {
  return clamp(roundCropPercent(value), 0, 100);
}

function parseCrop(value: string): ImageCrop | null {
  const parts = value.split(',').map((part) => Number(part.trim()));
  if (parts.length !== 4 || parts.some((part) => !Number.isFinite(part))) return null;
  const [left, top, right, bottom] = parts.map(clampPercent) as [number, number, number, number];
  if (right <= left || bottom <= top) return null;
  return { left, top, right, bottom };
}

function embedFrom(
  src: string,
  params: string[],
  kind: MediaKind,
  wiki: boolean,
): ImageEmbed {
  let width: number | null = null;
  let align: ImageAlign = DEFAULT_IMAGE_ALIGN;
  let crop: ImageCrop | null = null;

  for (const raw of params) {
    const param = raw.trim();
    if (!param) continue;
    if (/^\d+$/.test(param)) {
      const value = Number(param);
      if (value > 0) width = value;
      continue;
    }
    if (ALIGNS.has(param)) {
      align = param as ImageAlign;
      continue;
    }
    const cropParam = param.match(/^crop=(.+)$/);
    if (cropParam?.[1]) crop = parseCrop(cropParam[1]);
  }

  return { src, width, align, crop, kind, wiki };
}

export function parseImageEmbed(text: string): ImageEmbed | null {
  const line = text.trim();

  const wiki = line.match(WIKI_MEDIA_LINE);
  if (wiki) {
    const [target = '', ...params] = (wiki[1] ?? '').split('|');
    const src = target.trim();
    const kind = src ? mediaKindOf(src) : null;
    return kind ? embedFrom(src, params, kind, true) : null;
  }

  const match = line.match(IMAGE_LINE);
  if (!match) return null;
  const src = (match[2] ?? match[3] ?? '').trim();
  if (!src) return null;
  return embedFrom(src, (match[1] ?? '').split('|'), mediaKindOf(src) ?? 'image', false);
}

export function formatImageEmbed(embed: ImageEmbedInput): string {
  const params: string[] = [];
  if (embed.width) params.push(String(Math.round(embed.width)));
  if (embed.align !== DEFAULT_IMAGE_ALIGN) params.push(embed.align);
  if (embed.crop) {
    const { left, top, right, bottom } = embed.crop;
    params.push(`crop=${left},${top},${right},${bottom}`);
  }
  if (embed.wiki) return `![[${[embed.src, ...params].join('|')}]]`;
  const target = /\s/.test(embed.src) ? `<${embed.src}>` : embed.src;
  return `![${params.join('|')}](${target})`;
}

export function nextImageAlign(align: ImageAlign): ImageAlign {
  const at = IMAGE_ALIGN_ORDER.indexOf(align);
  return IMAGE_ALIGN_ORDER[(at + 1) % IMAGE_ALIGN_ORDER.length] ?? DEFAULT_IMAGE_ALIGN;
}

export function sameImageCrop(left: ImageCrop | null, right: ImageCrop | null): boolean {
  if (!left || !right) return left === right;
  return left.left === right.left
    && left.top === right.top
    && left.right === right.right
    && left.bottom === right.bottom;
}
