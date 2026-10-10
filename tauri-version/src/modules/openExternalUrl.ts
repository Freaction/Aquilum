import { openUrl } from '@tauri-apps/plugin-opener';

export function normalizeExternalUrl(raw: string): string | null {
  const trimmed = raw.trim().replace(/^<|>$/g, '');
  if (/^https?:\/\//i.test(trimmed) || /^mailto:/i.test(trimmed)) {
    return trimmed;
  }
  return null;
}

export async function openExternalUrl(raw: string): Promise<void> {
  const url = normalizeExternalUrl(raw);
  if (!url) return;
  await openUrl(url);
}
