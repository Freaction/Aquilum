import type { ShortcutToken } from '../../config/shortcuts';
import { createContext, useContext, useState, ReactNode, useCallback, useMemo, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { setTheme, type Theme } from '../theme';
import { setLanguage } from '../../i18n';
import { setFilesFolder } from '../docs/vaultFiles';
import { DEFAULT_DAILY_NOTES_SETTINGS, type DailyNotesSettings } from '../dailyNotes';
import { pxToRem } from '../scaling';
import { fontStack, IA_WRITER_QUATTRO, knownFont, type FontFamily } from '../../fonts/catalog';

export interface FontSettings {
  fontFamily: FontFamily;
  fontWeight: number;
  fontSizeBase: number;
}

interface Bm25fParams {
  k1: number;
  k3: number;
  bTitle: number;
  bBody: number;
  titleWeight: number;
}

interface AnalysisSettings {
  enableBm25f: boolean;
  enableAdamicAdar: boolean;
  enableWikixiv: boolean;
  bm25fParams: Bm25fParams;
}

interface SearchIndexSettings {
  candidatePoolSize: number;
  maxQueryTerms: number;
}

interface EditorSettings extends FontSettings {
  saveDebounceMs: number;
  lineHeight: number;
  maxWidthCh: number;
  fullWidth: boolean;
  fullWidthShortcut: ShortcutToken | null;
  smartDashes: boolean;
  listCallouts: boolean;
  autoLinkTitle: boolean;
  linkSuggest: boolean;
  linkSuggestMinChars: number;
  liveTabs: number;
}

export const DEFAULT_EDITOR_MAX_WIDTH_CH = 65;
export const DEFAULT_LIVE_TABS = 3;

export const DEFAULT_LINK_SUGGEST_MIN_CHARS = 2;

export type ReaderFlow = 'paginated' | 'scrolled';

export interface ReaderSettings extends FontSettings {
  lineHeight: number;
  maxWidthCh: number;
  marginPx: number;
  justify: boolean;
  hyphenate: boolean;
  flow: ReaderFlow;
}

export const DEFAULT_READER_SETTINGS: ReaderSettings = {
  fontFamily: IA_WRITER_QUATTRO,
  fontWeight: 400,
  fontSizeBase: 18,
  lineHeight: 1.6,
  maxWidthCh: 65,
  marginPx: 48,
  justify: true,
  hyphenate: true,
  flow: 'paginated',
};

interface UiSettings extends FontSettings {
  theme: Theme;
  language: string;
  primaryColor: string;
}

export interface McpSettings {
  enabled: boolean;
  port: number;
  token: string;
  allowWrite: boolean;
}

interface TrashSettings {
  retentionDays: number;
}

interface HistorySettings {
  retentionDays: number;
}

export interface PluginSettings {
  calendar: { enabled: boolean; openTodayShortcut: ShortcutToken | null; showWeekNumbers: boolean; weekly: { enabled: boolean; folder: string; format: string; template: string } };
  folderCounts: { enabled: boolean; showAllFiles: boolean; hideZero: boolean };
  fileColors: { enabled: boolean; cascade: boolean; background: boolean };
  fileIcons: { enabled: boolean };
  explorerFilters: { enabled: boolean };
  coloredTags: { enabled: boolean; mixNested: boolean; tagColors: Record<string, string> };
  cursorTrail: { enabled: boolean };
  codeStyler: { enabled: boolean; lineNumbers: boolean; copyButton: boolean; header: boolean };
  readingMode: { enabled: boolean; shortcut: ShortcutToken | null };
  advancedTables: { enabled: boolean; formatOnLeave: boolean };
  gitSync: { enabled: boolean; commitMessage: string; commitDateFormat: string; autoBackupMinutes: number; pullOnOpen: boolean; push: boolean; syncMethod: 'merge' | 'rebase' };
}

export const DEFAULT_PLUGIN_SETTINGS: PluginSettings = {
  calendar: { enabled: true, openTodayShortcut: null, showWeekNumbers: false, weekly: { enabled: false, folder: '', format: 'gggg-[W]ww', template: '' } },
  folderCounts: { enabled: false, showAllFiles: false, hideZero: true },
  fileColors: { enabled: false, cascade: false, background: false },
  fileIcons: { enabled: false },
  explorerFilters: { enabled: false },
  coloredTags: { enabled: false, mixNested: true, tagColors: {} },
  cursorTrail: { enabled: false },
  codeStyler: { enabled: false, lineNumbers: false, copyButton: true, header: true },
  readingMode: { enabled: false, shortcut: null },
  advancedTables: { enabled: false, formatOnLeave: false },
  gitSync: { enabled: false, commitMessage: 'vault backup: {{date}}', commitDateFormat: 'YYYY-MM-DD HH:mm:ss', autoBackupMinutes: 0, pullOnOpen: false, push: true, syncMethod: 'merge' },
};

export function withPluginDefaults(plugins?: PluginSettings): PluginSettings {
  return {
    calendar: { ...DEFAULT_PLUGIN_SETTINGS.calendar, ...plugins?.calendar,
      weekly: { ...DEFAULT_PLUGIN_SETTINGS.calendar.weekly, ...plugins?.calendar?.weekly } },
    folderCounts: { ...DEFAULT_PLUGIN_SETTINGS.folderCounts, ...plugins?.folderCounts },
    fileColors: { ...DEFAULT_PLUGIN_SETTINGS.fileColors, ...plugins?.fileColors },
    fileIcons: { ...DEFAULT_PLUGIN_SETTINGS.fileIcons, ...plugins?.fileIcons },
    explorerFilters: { ...DEFAULT_PLUGIN_SETTINGS.explorerFilters, ...plugins?.explorerFilters },
    coloredTags: { ...DEFAULT_PLUGIN_SETTINGS.coloredTags, ...plugins?.coloredTags,
      tagColors: { ...plugins?.coloredTags?.tagColors } },
    cursorTrail: { ...DEFAULT_PLUGIN_SETTINGS.cursorTrail, ...plugins?.cursorTrail },
    codeStyler: { ...DEFAULT_PLUGIN_SETTINGS.codeStyler, ...plugins?.codeStyler },
    readingMode: { ...DEFAULT_PLUGIN_SETTINGS.readingMode, ...plugins?.readingMode },
    advancedTables: { ...DEFAULT_PLUGIN_SETTINGS.advancedTables, ...plugins?.advancedTables },
    gitSync: { ...DEFAULT_PLUGIN_SETTINGS.gitSync, ...plugins?.gitSync },
  };
}

export interface AppConfig {
  analysis: AnalysisSettings;
  mcp: McpSettings;
  trash: TrashSettings;
  history: HistorySettings;
  search: SearchIndexSettings;
  editor: EditorSettings;
  reader: ReaderSettings;
  ui: UiSettings;
  templates: { folder: string };
  dailyNotes: DailyNotesSettings;
  plugins: PluginSettings;
  files: { folder: string };
  updates: { auto: boolean };
}

interface SettingsContextValue {
  config: AppConfig | null;
  isLoading: boolean;
  loadConfig: () => Promise<void>;
  updateConfig: (newConfig: AppConfig) => Promise<void>;
}

const SettingsContext = createContext<SettingsContextValue | null>(null);

function applyFont(root: HTMLElement, font: FontSettings, scope: 'ui' | 'editor') {
  const stack = fontStack(font.fontFamily);
  const weight = String(font.fontWeight);
  if (scope === 'ui') {
    root.style.setProperty('--q-font-family-ui', stack);
    root.style.setProperty('--q-font-weight-ui-base', weight);
    const size = font.fontSizeBase;
    root.style.setProperty('--q-font-size-ui-base', pxToRem(size));
    root.style.setProperty('--q-font-size-ui-sm', pxToRem(Math.round(size * (12 / 14))));
    root.style.setProperty('--q-font-size-ui-lg', pxToRem(Math.round(size * (16 / 14))));
    root.style.setProperty('--q-font-size-ui-xs', pxToRem(Math.round(size * (10 / 14))));
    return;
  }

  root.style.setProperty('--q-font-family-editor', stack);
  root.style.setProperty('--q-editor-font-weight', weight);
  root.style.setProperty('--q-editor-font-size', pxToRem(font.fontSizeBase));
}

function withKnownFonts(config: AppConfig): AppConfig {
  return {
    ...config,
    ui: knownFont('ui', config.ui),
    editor: { ...knownFont('editor', config.editor), fullWidthShortcut: config.editor.fullWidthShortcut ?? null },
    reader: knownFont('reader', config.reader),
    dailyNotes: { ...DEFAULT_DAILY_NOTES_SETTINGS, ...config.dailyNotes },
    plugins: withPluginDefaults(config.plugins),
  };
}

function applyPrimaryColor(root: HTMLElement, hex: string) {
  root.style.setProperty('--q-blue-alpha-main', hex);
  root.style.setProperty('--q-blue-500', hex);
  root.style.setProperty('--q-blue-600', `color-mix(in srgb, ${hex} 78%, black)`);
}

export function SettingsProvider({ children }: { children: ReactNode }) {
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const saveQueue = useRef(Promise.resolve());

  const loadConfig = useCallback(async () => {
    try {
      setIsLoading(true);
      const newConfig = withKnownFonts(await invoke<AppConfig>('get_settings'));
      applySettingsToDom(newConfig);
      setConfig(newConfig);
    } catch (err) {
      console.error('Failed to load settings:', err);
    } finally {
      setIsLoading(false);
    }
  }, []);

  const updateConfig = useCallback(async (newConfig: AppConfig) => {
    applySettingsToDom(newConfig);
    setConfig(newConfig);
    const save = saveQueue.current
      .catch(() => undefined)
      .then(() => invoke('update_settings', { newConfig }))
      .then(() => undefined);
    saveQueue.current = save;
    await save;
  }, []);

  const value = useMemo(() => ({
    config, isLoading, loadConfig, updateConfig,
  }), [config, isLoading, loadConfig, updateConfig]);

  return (
    <SettingsContext.Provider value={value}>
      {children}
    </SettingsContext.Provider>
  );
}

export function useSettingsStore() {
  const context = useContext(SettingsContext);
  if (!context) {
    throw new Error('useSettingsStore must be used within SettingsProvider');
  }
  return context;
}

function applySettingsToDom(config: AppConfig) {
  const root = document.documentElement;
  setLanguage(config.ui.language);
  setFilesFolder(config.files.folder);
  setTheme(config.ui.theme);
  applyFont(root, config.ui, 'ui');
  applyFont(root, config.editor, 'editor');
  root.style.setProperty('--q-editor-line-height', String(config.editor.lineHeight));
  root.style.setProperty(
    '--q-editor-max-width',
    config.editor.fullWidth ? 'none' : `${config.editor.maxWidthCh}ch`,
  );
  applyPrimaryColor(root, config.ui.primaryColor);
}
