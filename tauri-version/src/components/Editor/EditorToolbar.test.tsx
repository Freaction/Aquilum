// @vitest-environment happy-dom
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { act } from 'preact/test-utils';
import { EditorToolbar } from './EditorToolbar';
import { SidebarRail } from '../Layout/SidebarRail';
import { actAndSettle, mountDom, type MountedDom } from '../../testing/mountDom';
import { legacyConfig } from '../../plugins/testFixtures';
import { DEFAULT_PLUGIN_SETTINGS } from '../../modules/settings';
import { getReadingMode, setReadingMode } from '../../plugins/editor/readingMode';
import { formatShortcut, SHORTCUTS } from '../../config/shortcuts';
import { shortcutCatalog } from '../../config/shortcutCatalog';
import { t } from '../../i18n';

const state = vi.hoisted(() => ({ config: null as any, updateConfig: vi.fn() }));
vi.mock('../../modules/settings', async importOriginal => ({
  ...await importOriginal<typeof import('../../modules/settings')>(),
  useSettingsStore: () => state,
}));
let mounted: MountedDom | undefined;
const render = () => <><EditorToolbar fileName="note" canGoBack={false} canGoForward={false} onNavigate={() => {}} onSearch={() => {}} onExportPdf={() => {}} focusMode={false} onToggleFocusMode={() => {}} /><SidebarRail isSidebarOpen /></>;
const button = (label: string) => [...document.querySelectorAll<HTMLButtonElement>('button')].find(button => (button.querySelector('.q-menu__item-label')?.textContent ?? button.textContent) === label || button.getAttribute('aria-label') === label)!;
const menu = () => actAndSettle(() => button(t('editor.noteActions')).click());
beforeEach(() => {
  state.config = { ...legacyConfig(), plugins: structuredClone(DEFAULT_PLUGIN_SETTINGS) };
  state.updateConfig.mockReset().mockImplementation(async next => { state.config = next; mounted?.update(render()); });
  setReadingMode(false);
});
afterEach(() => { act(() => mounted?.unmount()); mounted = undefined; setReadingMode(false); });

it('offers both width actions and lists the global shortcut', async () => {
  await actAndSettle(() => { mounted = mountDom(render()); });
  await menu();
  await actAndSettle(() => button(t('editor.widthFull')).click());
  expect(state.config.editor.fullWidth).toBe(true);
  await menu();
  await actAndSettle(() => button(t('editor.widthReadable')).click());
  expect(state.config.editor.fullWidth).toBe(false);
  expect(shortcutCatalog().flatMap(group => group.items).find(item => item.label === t('shortcuts.toggleFullWidth'))?.shortcut).toEqual(SHORTCUTS.TOGGLE_FULL_WIDTH);
});

it('shows reading mode and its shortcut only when enabled', async () => {
  await actAndSettle(() => { mounted = mountDom(render()); });
  await menu();
  const label = t('plugins.readingMode.name');
  expect(button(label)).toBeUndefined();
  state.config.plugins.readingMode.enabled = true;
  await actAndSettle(() => mounted!.update(render()));
  expect(button(`${t('plugins.readingMode.enable')} (${formatShortcut(SHORTCUTS.TOGGLE_READING_MODE)})`)).toBeDefined();
  await actAndSettle(() => button(label).click());
  expect(getReadingMode()).toBe(true);
});

it('uses effective shortcuts in the menu, rail and catalog', async () => {
  const custom = { code: 'KeyJ', key: 'J', primary: true, alt: true, shift: false };
  state.config.editor.fullWidthShortcut = custom;
  state.config.plugins.readingMode.enabled = true;
  state.config.plugins.readingMode.shortcut = { ...custom, code: 'KeyK', key: 'K' };
  await actAndSettle(() => { mounted = mountDom(render()); });
  await menu();
  const menuRoot = document.querySelector<HTMLElement>('[role="menu"]')!;
  expect(menuRoot.style.width).toBe('');
  expect([...menuRoot.querySelectorAll('.q-menu__item-shortcut')].map(item => item.textContent)).toEqual([formatShortcut(custom), formatShortcut(state.config.plugins.readingMode.shortcut)]);
  expect(menuRoot.querySelector('.q-menu__item-label')!.textContent).toBe(t('editor.widthFull'));
  expect(button(`${t('plugins.readingMode.enable')} (${formatShortcut(state.config.plugins.readingMode.shortcut)})`)).toBeDefined();
  expect(shortcutCatalog(state.config).flatMap(group => group.items).find(item => item.label === t('shortcuts.toggleFullWidth'))!.shortcut).toEqual(custom);
});
