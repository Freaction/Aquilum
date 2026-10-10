// @vitest-environment happy-dom
import { act } from 'preact/test-utils';
import { afterEach, expect, it } from 'vitest';
import { mountDom, type MountedDom } from '../../testing/mountDom';
import { DEFAULT_PLUGIN_SETTINGS } from '../../modules/settings';
import { legacyConfig } from '../testFixtures';
import { CalendarSettings } from './CalendarSettings';
import { t } from '../../i18n';

let mounted: MountedDom | undefined;
afterEach(() => { mounted?.unmount(); mounted = undefined; });
it('updates week options without resetting daily notes or other plugin settings', () => {
  let config = { ...legacyConfig(), plugins: structuredClone(DEFAULT_PLUGIN_SETTINGS) };
  config.plugins.calendar.weekly.folder = 'Weekly';
  const render = () => <CalendarSettings config={config} workspacePath="/vault" onChange={next => { config = next; mounted!.update(render()); }} />;
  act(() => { mounted = mountDom(render()); });
  const toggle = (key: string) => mounted!.container.querySelector<HTMLButtonElement>(`[role="switch"][aria-label="${t(key)}"]`)!;
  act(() => toggle('plugins.calendar.showWeekNumbers').click());
  act(() => toggle('plugins.calendar.weeklyEnabled').click());
  expect(config.plugins.calendar.showWeekNumbers).toBe(true);
  expect(config.plugins.calendar.weekly).toEqual({ ...DEFAULT_PLUGIN_SETTINGS.calendar.weekly, enabled: true, folder: 'Weekly' });
  expect(config.dailyNotes).toEqual(legacyConfig().dailyNotes);
  expect(config.plugins.gitSync).toEqual(DEFAULT_PLUGIN_SETTINGS.gitSync);
  for (const key of ['weeklyFolder', 'weeklyFormat', 'weeklyTemplate']) {
    expect(mounted!.container.querySelector(`[aria-label="${t(`plugins.calendar.${key}`)}"]`)).not.toBeNull();
  }
});
