import { getCurrentWindow } from '@tauri-apps/api/window';

let cached: string | null = null;

export function getWindowId(): string {
  if (cached) return cached;
  try {
    cached = getCurrentWindow().label || 'main';
  } catch {
    cached = 'main';
  }
  return cached;
}
