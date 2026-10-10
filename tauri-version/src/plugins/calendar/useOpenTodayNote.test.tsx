// @vitest-environment happy-dom
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { actAndSettle, mountDom, type MountedDom } from '../../testing/mountDom';
import { useOpenTodayNote } from './useOpenTodayNote';
import { shortcutCatalog } from '../../config/shortcutCatalog';
import { t } from '../../i18n';
import { legacyConfig } from '../testFixtures';
import { DEFAULT_PLUGIN_SETTINGS } from '../../modules/settings';

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));
let mounted: MountedDom | undefined;
const onOpen = vi.fn();
beforeEach(() => { invoke.mockReset(); onOpen.mockReset(); });
afterEach(() => { mounted?.unmount(); mounted = undefined; });
function Harness({ workspace = '/vault', ready = true, enabled = true, config }: { workspace?: string | null; ready?: boolean; enabled?: boolean; config?: import('../../modules/settings').AppConfig }) {
  const error = useOpenTodayNote(workspace, ready, enabled, onOpen, config);
  return <output>{error}</output>;
}
function shortcut(modifiers = {}) {
  return new KeyboardEvent('keydown', { code: 'KeyD', key: 'в', ctrlKey: true, shiftKey: true, bubbles: true, cancelable: true, ...modifiers });
}
it.each([{ ctrlKey: true }, { ctrlKey: false, metaKey: true }])('opens today from the physical Mod+Shift+D shortcut %j', async modifiers => {
  invoke.mockResolvedValue({ path: '/vault/today.md', created: true });
  await actAndSettle(() => { mounted = mountDom(<Harness />); });
  const event = shortcut(modifiers);
  await actAndSettle(() => { window.dispatchEvent(event); });
  expect(event.defaultPrevented).toBe(true);
  expect(invoke).toHaveBeenCalledWith('calendar_open', expect.objectContaining({ workspacePath: '/vault', period: 'day', create: true, date: expect.stringMatching(/^\d{4}-\d{2}-\d{2}$/), now: expect.any(Object) }));
  expect(onOpen).toHaveBeenCalledWith('/vault/today.md');
  expect(shortcutCatalog()[0].items.some(item => item.label === t('shortcuts.openTodayNote'))).toBe(true);
});
it.each([{ workspace: null }, { ready: false }, { enabled: false }])('does not create without an active calendar context %j', async props => {
  await actAndSettle(() => { mounted = mountDom(<Harness {...props} />); });
  await actAndSettle(() => { window.dispatchEvent(shortcut()); });
  expect(invoke).not.toHaveBeenCalled();
});
it('ignores modified variants and stale results after switching workspaces', async () => {
  let resolve!: (note: { path: string; created: boolean }) => void;
  invoke.mockReturnValue(new Promise(done => { resolve = done; }));
  await actAndSettle(() => { mounted = mountDom(<Harness />); });
  await actAndSettle(() => { window.dispatchEvent(shortcut({ altKey: true })); });
  expect(invoke).not.toHaveBeenCalled();
  await actAndSettle(() => { window.dispatchEvent(shortcut()); });
  await actAndSettle(() => mounted!.update(<Harness workspace="/second" />));
  await actAndSettle(() => resolve({ path: '/vault/old.md', created: true }));
  expect(onOpen).not.toHaveBeenCalled();
});
it('shows a command failure and never opens a note', async () => {
  invoke.mockRejectedValue({ code: 'io' });
  await actAndSettle(() => { mounted = mountDom(<Harness />); });
  await actAndSettle(() => { window.dispatchEvent(shortcut()); });
  expect(mounted!.container.textContent).toBe('plugins.calendar.openTodayError');
  expect(onOpen).not.toHaveBeenCalled();
});

it('uses the configured shortcut and ignores the former default', async () => {
  const config = { ...legacyConfig(), plugins: structuredClone(DEFAULT_PLUGIN_SETTINGS) };
  config.plugins.calendar.openTodayShortcut = { code: 'KeyJ', key: 'J', primary: true, alt: true, shift: false };
  invoke.mockResolvedValue({ path: '/vault/today.md', created: true });
  await actAndSettle(() => { mounted = mountDom(<Harness config={config} />); });
  await actAndSettle(() => { window.dispatchEvent(shortcut()); });
  expect(onOpen).not.toHaveBeenCalled();
  await actAndSettle(() => { window.dispatchEvent(shortcut({ code: 'KeyJ', key: 'о', shiftKey: false, altKey: true })); });
  expect(onOpen).toHaveBeenCalledWith('/vault/today.md');
});
