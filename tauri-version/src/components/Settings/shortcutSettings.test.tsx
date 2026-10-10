// @vitest-environment happy-dom
import { afterEach, expect, it, vi } from 'vitest';
import { actAndSettle, mountDom, type MountedDom } from '../../testing/mountDom';
import { legacyConfig } from '../../plugins/testFixtures';
import { DEFAULT_PLUGIN_SETTINGS, type AppConfig } from '../../modules/settings';
import { EditorSection } from './sections/EditorSection';
import { ShortcutsSection } from './sections/ShortcutsSection';
import { CalendarSettings } from '../../plugins/calendar/CalendarSettings';
import { ReadingModeSettings } from '../../plugins/editor/ReadingModeSettings';
import { PLUGINS } from '../../plugins/registry';
import { setLanguage, t } from '../../i18n';

const state = vi.hoisted(() => ({ config: null as AppConfig | null }));
vi.mock('../../modules/settings', async original => ({
  ...await original<typeof import('../../modules/settings')>(), useSettingsStore: () => state,
}));
let mounted: MountedDom;
afterEach(() => { mounted?.unmount(); state.config = null; setLanguage('en'); vi.restoreAllMocks(); });

it.each(['en', 'ru'])('saves all three fields from their settings sections (%s)', async language => {
  setLanguage(language);
  let config = { ...legacyConfig(), plugins: structuredClone(DEFAULT_PLUGIN_SETTINGS) };
  const change = (next: AppConfig) => { config = next; mounted.update(render()); };
  const render = () => <><EditorSection config={config} onChange={change} /><CalendarSettings config={config} workspacePath="/vault" onChange={change} /><ReadingModeSettings config={config} workspacePath="/vault" onChange={change} /></>;
  await actAndSettle(() => { mounted = mountDom(render()); });
  const fields = [
    ['settings.editor.fullWidthShortcut', 'KeyJ', () => config.editor.fullWidthShortcut],
    ['plugins.calendar.openTodayShortcut', 'KeyK', () => config.plugins.calendar.openTodayShortcut],
    ['plugins.readingMode.shortcut', 'KeyL', () => config.plugins.readingMode.shortcut],
  ] as const;
  for (const [label, code, getValue] of fields) {
    await actAndSettle(() => mounted.container.querySelector<HTMLButtonElement>(`button[aria-label="${t(label)}"]`)!.click());
    await actAndSettle(() => { window.dispatchEvent(new KeyboardEvent('keydown', { code, key: 'о', metaKey: true, altKey: true, cancelable: true })); });
    expect(getValue()).toEqual({ code, key: code.slice(3), primary: true, alt: true, shift: false });
  }
  expect(config.editor.fullWidth).toBe(false);
  expect(config.plugins.calendar.weekly).toEqual(DEFAULT_PLUGIN_SETTINGS.calendar.weekly);
  expect(PLUGINS.find(plugin => plugin.id === 'readingMode')!.settings).toBe(ReadingModeSettings);
});

it.each(['en', 'ru'])('renders configured combinations in Keyboard shortcuts on macOS (%s)', async language => {
  setLanguage(language);
  vi.spyOn(navigator, 'platform', 'get').mockReturnValue('MacIntel');
  state.config = { ...legacyConfig(), plugins: structuredClone(DEFAULT_PLUGIN_SETTINGS) };
  state.config.editor.fullWidthShortcut = { code: 'KeyJ', key: 'J', primary: true, alt: true, shift: false };
  state.config.plugins.calendar.openTodayShortcut = { code: 'KeyK', key: 'K', primary: true, alt: false, shift: true };
  state.config.plugins.readingMode.shortcut = { code: 'KeyL', key: 'L', primary: false, alt: true, shift: false };
  await actAndSettle(() => { mounted = mountDom(<ShortcutsSection />); });
  const rows = [...mounted.container.querySelectorAll('.q-settings-row')];
  for (const [label, shortcut] of [['shortcuts.toggleFullWidth', '⌘⌥J'], ['shortcuts.openTodayNote', '⌘⇧K'], ['shortcuts.toggleReadingMode', '⌥L']]) {
    const row = rows.find(row => row.textContent!.includes(t(label)))!;
    expect(row.querySelector('kbd')!.textContent).toBe(shortcut);
  }
});
