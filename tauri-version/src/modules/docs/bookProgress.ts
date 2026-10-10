import { clamp } from '../math';

const BYTES_PER_PAGE = 1024;

export type Pages = { current: number; total: number };

export function pagesFromByteLength(byteLength: number): number {
  return Math.max(1, Math.ceil(Math.max(0, byteLength) / BYTES_PER_PAGE));
}

export function parsePages(value: string | undefined): Pages | null {
  if (!value) return null;
  const match = value.trim().match(/^(\d+)\s*\/\s*(\d+)$/);
  if (!match) return null;
  const current = Number(match[1]);
  const total = Number(match[2]);
  if (!Number.isFinite(current) || !Number.isFinite(total) || total <= 0) return null;
  return { current: Math.max(0, current), total };
}

export function formatPages(current: number, total: number): string {
  const safeTotal = Math.max(1, Math.round(Number.isFinite(total) ? total : 1));
  const raw = Number.isFinite(current) ? current : 0;
  const safeCurrent = clamp(Math.round(raw), 0, safeTotal);
  return `${safeCurrent}/${safeTotal}`;
}

export function currentFromFraction(fraction: number, total: number): number {
  if (!Number.isFinite(fraction) || !Number.isFinite(total)) return 0;
  const safeTotal = Math.max(1, total);
  const f = clamp(fraction, 0, 1);
  return clamp(Math.round(f * safeTotal), 0, safeTotal);
}

export function normalizeToSyntheticTotal(
  existing: string | undefined,
  syntheticTotal: number,
): Pages {
  const total = Math.max(1, syntheticTotal);
  const parsed = parsePages(existing);
  if (!parsed) return { current: 0, total };
  return {
    current: currentFromFraction(parsed.current / parsed.total, total),
    total,
  };
}

export function formatSyntheticPages(
  existing: string | undefined,
  byteLength: number,
): { pages: Pages; formatted: string } {
  const pages = normalizeToSyntheticTotal(existing, pagesFromByteLength(byteLength));
  return { pages, formatted: formatPages(pages.current, pages.total) };
}
