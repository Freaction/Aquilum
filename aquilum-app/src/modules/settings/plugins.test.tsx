// @vitest-environment happy-dom
import { useEffect } from 'react';
import { act } from 'preact/test-utils';
import { afterEach, expect, it, vi } from 'vitest';
import { SettingsProvider, useSettingsStore, type AppConfig } from './index';
import { legacyConfig } from '../../plugins/testFixtures';
import { mountDom, actAndSettle, type MountedDom } from '../../testing/mountDom';

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));

let config: AppConfig | null;
function Harness() {
  const store = useSettingsStore();
  config = store.config;
  useEffect(() => { void store.loadConfig(); }, [store.loadConfig]);
  return null;
}

let mounted: MountedDom | undefined;
afterEach(() => {
  act(() => mounted?.unmount());
  mounted = undefined;
  config = null;
  vi.clearAllMocks();
  document.documentElement.removeAttribute('style');
});

it('loads a legacy config with Calendar enabled and all other plugins disabled', async () => {
  invoke.mockResolvedValue(legacyConfig());
  await actAndSettle(() => { mounted = mountDom(<SettingsProvider><Harness /></SettingsProvider>); });
  expect(config!.plugins.calendar).toEqual({ enabled: true, openTodayShortcut: null, showWeekNumbers: false,
    weekly: { enabled: false, folder: '', format: 'gggg-[W]ww', template: '' } });
  expect(Object.entries(config!.plugins).filter(([, settings]) => settings.enabled).map(([id]) => id)).toEqual(['calendar']);
  expect(Object.keys(config!.plugins)).toHaveLength(11);
  expect(config!.dailyNotes).toEqual(legacyConfig().dailyNotes);
});

it('deeply merges partial plugin settings without losing saved options', async () => {
  invoke.mockResolvedValue({ ...legacyConfig(), plugins: {
    gitSync: { enabled: true }, calendar: { weekly: { folder: 'Weekly' } },
    coloredTags: { tagColors: { work: 'blue' } },
  } });
  await actAndSettle(() => { mounted = mountDom(<SettingsProvider><Harness /></SettingsProvider>); });
  expect(config!.plugins.gitSync).toEqual({ enabled: true, commitMessage: 'vault backup: {{date}}',
    commitDateFormat: 'YYYY-MM-DD HH:mm:ss', autoBackupMinutes: 0, pullOnOpen: false, push: true, syncMethod: 'merge' });
  expect(config!.plugins.calendar.weekly).toEqual({ enabled: false, folder: 'Weekly', format: 'gggg-[W]ww', template: '' });
  expect(config!.plugins.calendar.enabled).toBe(true);
  expect(config!.plugins.coloredTags).toEqual({ enabled: false, mixNested: true, tagColors: { work: 'blue' } });
  expect(config!.plugins.folderCounts).toEqual({ enabled: false, showAllFiles: false, hideZero: true });
});
