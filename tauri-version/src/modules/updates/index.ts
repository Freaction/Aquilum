import { Channel, invoke } from '@tauri-apps/api/core';
import { useSyncExternalStore } from 'react';

export const UPDATES_UNAVAILABLE = 'unavailable';

type UpdateProgressEvent =
  | { phase: 'downloading'; chunk: number; total: number | null }
  | { phase: 'installing' };

export type UpdateProgress =
  | { phase: 'downloading'; downloaded: number; total: number | null }
  | { phase: 'installing' };

let progress: UpdateProgress | null = null;
let startupInstallRequested = false;
const listeners = new Set<() => void>();

function publish(next: UpdateProgress | null): void {
  progress = next;
  listeners.forEach((listener) => listener());
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

function applyProgressEvent(event: UpdateProgressEvent): void {
  if (event.phase === 'installing') {
    publish({ phase: 'installing' });
    return;
  }
  const downloaded = progress?.phase === 'downloading' ? progress.downloaded : 0;
  publish({ phase: 'downloading', downloaded: downloaded + event.chunk, total: event.total });
}

export function useUpdateProgress(): UpdateProgress | null {
  return useSyncExternalStore(subscribe, () => progress);
}

export function checkForUpdate(): Promise<string | null> {
  return invoke<string | null>('check_for_update');
}

export async function installUpdate(): Promise<void> {
  const onProgress = new Channel<UpdateProgressEvent>();
  onProgress.onmessage = applyProgressEvent;
  try {
    await invoke<void>('install_update', { onProgress });
  } finally {
    publish(null);
  }
}

export function installUpdateOnStartup(auto: boolean): void {
  if (startupInstallRequested) return;
  startupInstallRequested = true;
  if (!auto) return;
  installUpdate().catch((reason) => {
    console.warn('[aquilum:updater] автообновление пропущено:', reason);
  });
}
