// @vitest-environment happy-dom
import { useEffect } from 'react';
import { act } from 'preact/test-utils';
import { afterEach, expect, it, vi } from 'vitest';
import { SidebarRail } from '../../components/Layout/SidebarRail';
import { SettingsProvider, useSettingsStore, DEFAULT_PLUGIN_SETTINGS } from '../../modules/settings';
import { legacyConfig } from '../testFixtures';
import { setReadingMode, getReadingMode } from './readingMode';
import { mountDom, actAndSettle, type MountedDom } from '../../testing/mountDom';
import { t } from '../../i18n';
import { formatShortcut, matchesShortcut, SHORTCUTS } from '../../config/shortcuts';

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));
let mounted: MountedDom;
afterEach(() => { act(() => mounted?.unmount()); setReadingMode(false); vi.clearAllMocks(); });
it('shows an accessible pressed indicator and toggles the global mode', async () => {
  const config = { ...legacyConfig(), plugins: structuredClone(DEFAULT_PLUGIN_SETTINGS) };
  config.plugins.readingMode.enabled = true;
  invoke.mockResolvedValue(config);
  function Harness() {
    const { loadConfig } = useSettingsStore();
    useEffect(() => { void loadConfig(); }, [loadConfig]);
    return <SidebarRail isSidebarOpen={false} />;
  }
  await actAndSettle(() => { mounted = mountDom(<SettingsProvider><Harness /></SettingsProvider>); });
  const button = () => mounted.container.querySelector<HTMLButtonElement>('[aria-pressed]')!;
  expect(button().getAttribute('aria-label')).toBe(`${t('plugins.readingMode.enable')} (${formatShortcut(SHORTCUTS.TOGGLE_READING_MODE)})`);
  expect(button().getAttribute('aria-pressed')).toBe('false');
  await actAndSettle(() => button().click());
  expect(getReadingMode()).toBe(true);
  expect(button().getAttribute('aria-pressed')).toBe('true');
  expect(button().getAttribute('aria-label')).toBe(`${t('plugins.readingMode.disable')} (${formatShortcut(SHORTCUTS.TOGGLE_READING_MODE)})`);
  await actAndSettle(() => button().click());
  expect(getReadingMode()).toBe(false);
  expect(button().getAttribute('aria-pressed')).toBe('false');
});
it('binds the physical Mod+Shift+E shortcut for Ctrl and Meta', () => {
  for (const modifier of ['ctrlKey', 'metaKey']) {
    expect(matchesShortcut(new KeyboardEvent('keydown', { code: 'KeyE', key: 'E', shiftKey: true, [modifier]: true }), SHORTCUTS.TOGGLE_READING_MODE)).toBe(true);
  }
  expect(matchesShortcut(new KeyboardEvent('keydown', { code: 'KeyE', key: 'E', ctrlKey: true }), SHORTCUTS.TOGGLE_READING_MODE)).toBe(false);
});
