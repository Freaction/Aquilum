import { invoke } from '@tauri-apps/api/core';
import { isImageSource } from './docs/imageEmbeds';

const BARE_URL =
  /^(https?:\/\/(?:www\.|(?!www))[a-zA-Z0-9][a-zA-Z0-9-]+[a-zA-Z0-9]\.[^\s]{2,}|www\.[a-zA-Z0-9][a-zA-Z0-9-]+[a-zA-Z0-9]\.[^\s]{2,}|https?:\/\/(?:www\.|(?!www))[a-zA-Z0-9]+\.[^\s]{2,}|www\.[a-zA-Z0-9]+\.[^\s]{2,})$/i;

export function isBareHttpUrl(text: string): boolean {
  const trimmed = text.trim();
  if (!trimmed || /[\r\n]/.test(trimmed)) return false;
  return BARE_URL.test(trimmed);
}

export function isImageUrl(text: string): boolean {
  return isImageSource(text.trim());
}

export function isMarkdownLinkContext(beforeCursor: string): boolean {
  return beforeCursor.endsWith('](') || /\]\([^)\s]*$/.test(beforeCursor);
}

export function escapeMarkdownTitle(text: string): string {
  const unescaped = text.replace(/\\(\*|_|`|~|\\|\[|\])/g, '$1');
  return unescaped.replace(/(\*|_|`|\||<|>|~|\\|\[|\])/g, '\\$1');
}

export function hostnameFallback(url: string): string {
  const withoutScheme = url.replace(/^https?:\/\//i, '');
  const host = withoutScheme.split(/[/?#]/)[0]?.split('@').pop() ?? url;
  return host || url;
}

export function normalizeFetchUrl(raw: string): string {
  const trimmed = raw.trim();
  if (/^https?:\/\//i.test(trimmed)) return trimmed;
  if (/^www\./i.test(trimmed)) return `https://${trimmed}`;
  return trimmed;
}

let pasteSeq = 0;

export function makeTitlePlaceholder(): string {
  pasteSeq = (pasteSeq + 1) % 1_000_000;
  const id = `${Date.now().toString(36)}${pasteSeq.toString(36)}`;
  return `Fetching Title#${id}`;
}

export function markdownLink(title: string, url: string): string {
  return `[${title}](${url})`;
}

export async function fetchPageTitle(url: string): Promise<string> {
  try {
    const title = await invoke<string>('fetch_page_title', {
      url: normalizeFetchUrl(url),
    });
    const cleaned = title.replace(/[\r\n]+/g, ' ').trim();
    return cleaned || hostnameFallback(url);
  } catch {
    return hostnameFallback(url);
  }
}
