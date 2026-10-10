import { useSyncExternalStore } from 'react';

let active = false;
const listeners = new Set<() => void>();
export const getReadingMode = () => active;
export function subscribeReadingMode(listener: () => void): () => void {
  listeners.add(listener);
  return () => { listeners.delete(listener); };
}
export function setReadingMode(next: boolean): void {
  if (next === active) return;
  active = next;
  for (const listener of listeners) listener();
}
export const toggleReadingMode = () => setReadingMode(!active);
export const useReadingMode = () => useSyncExternalStore(subscribeReadingMode, getReadingMode, getReadingMode);
