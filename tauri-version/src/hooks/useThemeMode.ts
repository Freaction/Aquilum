import { useSyncExternalStore } from 'react';
import { subscribeThemeMode, themeMode } from '../modules/theme';

export function useThemeMode(): 'light' | 'dark' {
  return useSyncExternalStore(subscribeThemeMode, themeMode);
}
