// @vitest-environment happy-dom
import { act } from 'preact/test-utils';
import { afterEach, expect, it } from 'vitest';
import { DailyNotesSection } from './DailyNotesSection';
import { DEFAULT_DAILY_NOTES_SETTINGS } from '../../../modules/dailyNotes';
import type { AppConfig } from '../../../modules/settings';
import { mountDom, type MountedDom } from '../../../testing/mountDom';
import { t } from '../../../i18n';

let mounted: MountedDom | undefined;
afterEach(() => { act(() => mounted?.unmount()); mounted = undefined; });

it('updates daily note options without resetting the folder or other settings', () => {
  let config = { dailyNotes: { ...DEFAULT_DAILY_NOTES_SETTINGS, folder: 'Daily', wordsPerDot: 100 } } as AppConfig;
  const render = () => <DailyNotesSection config={config} onChange={next => { config = next; mounted!.update(render()); }} />;
  act(() => { mounted = mountDom(render()); });
  const startup = () => mounted!.container.querySelector<HTMLButtonElement>(`[role="switch"][aria-label="${t('settings.dailyNotes.openOnStartup')}"]`);
  expect(startup()).not.toBeNull();
  act(() => startup()!.click());
  expect(config.dailyNotes).toEqual({ ...DEFAULT_DAILY_NOTES_SETTINGS, folder: 'Daily', wordsPerDot: 100, openOnStartup: true });
  act(() => Array.from(mounted!.container.querySelectorAll<HTMLButtonElement>('button')).find(button => button.textContent === t('settings.dailyNotes.monday'))!.click());
  expect(config.dailyNotes.weekStart).toBe('monday');
  expect(config.dailyNotes.folder).toBe('Daily');
  expect(startup()!.getAttribute('aria-checked')).toBe('true');
});
