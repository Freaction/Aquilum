import { DEFAULT_DAILY_NOTES_SETTINGS } from '../modules/dailyNotes';
import { DEFAULT_READER_SETTINGS, type AppConfig } from '../modules/settings';

export function legacyConfig(): Omit<AppConfig, 'plugins'> {
  return {
    analysis: { enableBm25f: true, enableAdamicAdar: true, enableWikixiv: true,
      bm25fParams: { k1: 1.2, k3: 8, bTitle: 0.75, bBody: 0.75, titleWeight: 2 } },
    mcp: { enabled: false, port: 8787, token: '', allowWrite: false },
    trash: { retentionDays: 30 }, history: { retentionDays: 0 },
    search: { candidatePoolSize: 200, maxQueryTerms: 20 },
    editor: { fontFamily: DEFAULT_READER_SETTINGS.fontFamily, fontWeight: 400, fontSizeBase: 18,
      saveDebounceMs: 1000, lineHeight: 1.8, maxWidthCh: 80, fullWidth: false, fullWidthShortcut: null, smartDashes: true,
      listCallouts: true, autoLinkTitle: true, linkSuggest: true, linkSuggestMinChars: 2, liveTabs: 3 },
    reader: { ...DEFAULT_READER_SETTINGS },
    ui: { fontFamily: DEFAULT_READER_SETTINGS.fontFamily, fontWeight: 400, fontSizeBase: 14,
      theme: 'dark', language: 'en', primaryColor: 'currentColor' },
    templates: { folder: '' }, files: { folder: '' }, updates: { auto: false },
    dailyNotes: { ...DEFAULT_DAILY_NOTES_SETTINGS },
  };
}
