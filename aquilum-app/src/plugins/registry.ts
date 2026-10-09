import type { ComponentType } from 'react';
import { CalendarSettings } from './calendar/CalendarSettings';
import { CalendarDays, FolderClosed, Palette, Shapes, ListFilter, Tags, MousePointer2, Code, BookOpen, Table, GitBranch, type IconNode } from 'lucide';
import { FolderCountsSettings } from './explorer/FolderCountsSettings';
import { FileColorsSettings } from './explorer/FileColorsSettings';
import { ExplorerFiltersSettings } from './explorer/ExplorerFiltersSettings';
import { useSettingsStore, DEFAULT_PLUGIN_SETTINGS, type AppConfig, type PluginSettings } from '../modules/settings';
import { GitSettings } from './git/GitSettings';

import { ColoredTagsSettings } from './editor/ColoredTagsSettings';
import { ReadingModeSettings } from './editor/ReadingModeSettings';
import { CodeStylerSettings } from './editor/CodeStylerSettings';
import { AdvancedTablesSettings } from './editor/AdvancedTablesSettings';

export type PluginId = keyof PluginSettings;

export interface PluginSettingsProps {
  config: AppConfig;
  onChange: (next: AppConfig) => void;
  workspacePath: string | null;
}

export interface PluginDescriptor {
  id: PluginId;
  icon: IconNode;
  settings?: ComponentType<PluginSettingsProps>;
}

export const PLUGINS: PluginDescriptor[] = [
  { id: 'calendar', icon: CalendarDays, settings: CalendarSettings },
  { id: 'folderCounts', icon: FolderClosed, settings: FolderCountsSettings },
  { id: 'fileColors', icon: Palette, settings: FileColorsSettings },
  { id: 'fileIcons', icon: Shapes },
  { id: 'explorerFilters', icon: ListFilter, settings: ExplorerFiltersSettings },
  { id: 'coloredTags', icon: Tags, settings: ColoredTagsSettings },
  { id: 'cursorTrail', icon: MousePointer2 },
  { id: 'codeStyler', icon: Code, settings: CodeStylerSettings },
  { id: 'readingMode', icon: BookOpen, settings: ReadingModeSettings },
  { id: 'advancedTables', icon: Table, settings: AdvancedTablesSettings },
  { id: 'gitSync', icon: GitBranch, settings: GitSettings },
];

export function pluginSettings<Id extends PluginId>(config: AppConfig, id: Id): PluginSettings[Id] {
  return config.plugins[id];
}

export function usePluginEnabled(id: PluginId): boolean {
  const { config } = useSettingsStore();
  return config ? pluginSettings(config, id).enabled : DEFAULT_PLUGIN_SETTINGS[id].enabled;
}
