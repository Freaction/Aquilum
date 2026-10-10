import { useCallback } from 'react';
import {
  useSettingsStore,
  type AppConfig,
} from '../../modules/settings';

export function useSettingsPersist() {
  const { config, isLoading, updateConfig } = useSettingsStore();

  const persist = useCallback((next: AppConfig): Promise<boolean> => (
    updateConfig(next).then(
      () => true,
      (error) => {
        console.error('Failed to save settings', error);
        return false;
      },
    )
  ), [updateConfig]);

  return { config, isLoading, persist };
}
