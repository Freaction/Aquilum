import { clamp } from '../math';

const FRONTMATTER_RE = /^---\n([\s\S]*?)\n---/;

export const FM_BOOK_COVER = 'Book_cover';
export const FM_PAGE_COVER = 'Page_cover';
export const FM_BOOK_FILE = 'Путь к файлу';
export const FM_READER_POSITION = 'reader_position';
export const FM_READ_PERCENT = 'read_percent';
export const FM_PAGE_COVER_POSITION = 'page_cover_position';

const LEGACY_ALIASES: Record<string, readonly string[]> = {
  [FM_BOOK_COVER]: ['cover_url'],
  [FM_PAGE_COVER]: ['page_cover_url'],
  [FM_BOOK_FILE]: ['book_file'],
};

export interface BookMetadata {
  type?: string;
  author?: string;
  status?: string;
  pages?: string;
  tags?: string[];
  rating?: string;
  [FM_BOOK_COVER]?: string;
  cover_url?: string;
  [FM_BOOK_FILE]?: string;
  book_file?: string;
  cover?: string;
  [FM_PAGE_COVER]?: string;
  page_cover_url?: string;
  [FM_PAGE_COVER_POSITION]?: string;
  [FM_READER_POSITION]?: string;
  [FM_READ_PERCENT]?: string;
  [key: string]: string | string[] | undefined;
}

export interface ResolvedBookFields {
  bookCoverUrl?: string;
  pageCoverUrl?: string;
  pageCoverPosition?: string;
  bookFile?: string;
  readerPosition?: string;
  readPercent?: number;
}

export function parseReadPercent(value: string | undefined): number | undefined {
  if (!value) return undefined;
  const trimmed = value.trim().replace(/%$/, '');
  const n = Number(trimmed);
  if (!Number.isFinite(n)) return undefined;
  return clamp(Math.round(n), 0, 100);
}

export function formatReadPercent(fraction: number): string {
  const n = clamp(Math.round(fraction * 100), 0, 100);
  return String(n);
}

export function resolveBookFields(data: BookMetadata | null | undefined): ResolvedBookFields {
  if (!data) return {};
  return {
    bookCoverUrl: pickString(data, FM_BOOK_COVER),
    pageCoverUrl: pickString(data, FM_PAGE_COVER),
    pageCoverPosition: pickString(data, FM_PAGE_COVER_POSITION),
    bookFile: pickString(data, FM_BOOK_FILE),
    readerPosition: pickString(data, FM_READER_POSITION),
    readPercent: parseReadPercent(pickString(data, FM_READ_PERCENT)),
  };
}

function fieldKeys(key: string): readonly string[] {
  return [key, ...(LEGACY_ALIASES[key] ?? [])];
}

function pickString(data: BookMetadata, field: string): string | undefined {
  for (const key of fieldKeys(field)) {
    const value = data[key];
    if (typeof value === 'string' && value.trim()) return value.trim();
  }
  return undefined;
}

export function hasPageCover(data: BookMetadata): boolean {
  return typeof data.cover === 'string' && data.cover.trim().toLowerCase() === 'true';
}

export function parseFrontmatter(doc: string): { raw: string; body: string; data: BookMetadata } | null {
  const match = doc.match(FRONTMATTER_RE);
  if (!match) return null;
  const raw = match[0];
  const body = match[1];
  const data: BookMetadata = {};
  for (const line of body.split('\n')) {
    const { key, value: val } = splitFieldLine(line);
    if (!key || val === null) continue;
    if (val.startsWith('[') && val.endsWith(']')) {
      data[key] = val.slice(1, -1).split(',').map(s => s.trim()).filter(Boolean);
    } else {
      let cleanVal = val;
      if (
        (cleanVal.startsWith('"') && cleanVal.endsWith('"')) ||
        (cleanVal.startsWith("'") && cleanVal.endsWith("'"))
      ) {
        cleanVal = cleanVal.slice(1, -1);
      }
      data[key] = cleanVal;
    }
  }
  return { raw, body, data };
}

function splitFieldLine(line: string): { key: string; value: string | null } {
  const colonIdx = line.indexOf(':');
  if (colonIdx !== -1) {
    return { key: line.slice(0, colonIdx).trim(), value: line.slice(colonIdx + 1).trim() };
  }
  const trimmed = line.trim();
  const spaceIdx = trimmed.indexOf(' ');
  if (spaceIdx === -1) return { key: trimmed, value: null };
  return { key: trimmed.slice(0, spaceIdx).trim(), value: trimmed.slice(spaceIdx + 1).trim() };
}

export function setFrontmatterField(doc: string, key: string, value: string): string | null {
  const parsed = parseFrontmatter(doc);
  if (!parsed) return null;

  const dropKeys = new Set(fieldKeys(key));
  const lines = parsed.body.length > 0 ? parsed.body.split('\n') : [];
  let found = false;
  const nextLines = lines.flatMap((line) => {
    if (!dropKeys.has(splitFieldLine(line).key)) return [line];
    found = true;
    if (value === '') return [];
    return [`${key}: ${value}`];
  });
  if (!found && value !== '') nextLines.push(`${key}: ${value}`);

  return `---\n${nextLines.join('\n')}\n---${doc.slice(parsed.raw.length)}`;
}

export function frontmatterRange(doc: string): { from: number; to: number } | null {
  const match = doc.match(FRONTMATTER_RE);
  if (!match) return null;
  return { from: 0, to: match[0].length };
}

export function noteFrontmatterTemplate(): string {
  return ['---', 'tags: []', 'author: ', 'source: ', '---'].join('\n');
}
