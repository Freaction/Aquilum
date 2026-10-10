// @vitest-environment happy-dom
import { afterEach, expect, it, vi } from 'vitest';
import { actAndSettle, mountDom, type MountedDom } from '../../../testing/mountDom';
import { legacyConfig } from '../../../plugins/testFixtures';
import { DEFAULT_PLUGIN_SETTINGS } from '../../../modules/settings';
import { ShortcutInput } from './ShortcutInput';
import { t } from '../../../i18n';

let mounted: MountedDom;
afterEach(() => mounted?.unmount());
async function create() {
  const config = { ...legacyConfig(), plugins: structuredClone(DEFAULT_PLUGIN_SETTINGS) };
  const change = vi.fn();
  await actAndSettle(() => { mounted = mountDom(<ShortcutInput config={config} id="TOGGLE_FULL_WIDTH" value={null} label="Width shortcut" onChange={change} />); });
  const record = () => actAndSettle(() => mounted.container.querySelector<HTMLButtonElement>('button[aria-label="Width shortcut"]')!.click());
  const press = (options: KeyboardEventInit) => actAndSettle(() => { window.dispatchEvent(new KeyboardEvent('keydown', { bubbles: true, cancelable: true, ...options })); });
  return { config, change, record, press };
}
it('records the first modified physical key and isolates it from global handlers', async () => {
  const { change, record, press } = await create();
  const global = vi.fn();
  window.addEventListener('keydown', global);
  try {
    await record();
    expect(mounted.container.querySelector('[aria-live="polite"]')!.textContent).toBe(t('settings.shortcuts.recording'));
    await press({ code: 'ShiftLeft', key: 'Shift', shiftKey: true });
    await press({ code: 'KeyJ', key: 'о' });
    expect(change).not.toHaveBeenCalled();
    await press({ code: 'KeyJ', key: 'о', metaKey: true, altKey: true });
    expect(change).toHaveBeenCalledWith({ code: 'KeyJ', key: 'J', primary: true, alt: true, shift: false });
    expect(global).not.toHaveBeenCalled();
    expect(mounted.container.querySelector('[aria-pressed="true"]')).toBeNull();
  } finally { window.removeEventListener('keydown', global); }
});
it('cancels with Escape without changing the value', async () => {
  const { change, record, press } = await create();
  await record();
  await press({ code: 'Escape', key: 'Escape' });
  expect(change).not.toHaveBeenCalled();
  expect(mounted.container.querySelector('[aria-pressed="true"]')).toBeNull();
});
it('resets the stored shortcut to null', async () => {
  const { change } = await create();
  await actAndSettle(() => [...mounted.container.querySelectorAll<HTMLButtonElement>('button')].find(button => button.textContent === t('settings.shortcuts.reset'))!.click());
  expect(change).toHaveBeenCalledWith(null);
});
it('rejects fixed and configured conflicts and allows correcting the combination', async () => {
  const { config, change, record, press } = await create();
  config.plugins.readingMode.shortcut = { code: 'KeyJ', key: 'J', primary: true, alt: true, shift: false };
  await record();
  await press({ code: 'KeyO', key: 'щ', ctrlKey: true });
  expect(change).not.toHaveBeenCalled();
  expect(mounted.container.querySelector('[role="alert"]')!.textContent).toContain(t('settings.shortcuts.conflict'));
  await press({ code: 'KeyJ', key: 'о', ctrlKey: true, altKey: true });
  expect(change).not.toHaveBeenCalled();
  await press({ code: 'KeyK', key: 'л', altKey: true });
  expect(change).toHaveBeenCalledWith({ code: 'KeyK', key: 'K', primary: false, alt: true, shift: false });
});
