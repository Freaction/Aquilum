import type { AppConfig } from '../../modules/settings';

export type SettingsSectionId = 'ui' | 'editor' | 'reader' | 'search' | 'dailyNotes' | 'templates' | 'files' | 'analysis' | 'plugins' | 'mcp' | 'history' | 'trash' | 'system' | 'shortcuts';

export interface SettingsSectionProps {
  config: AppConfig;
  onChange: (next: AppConfig) => void;
}
