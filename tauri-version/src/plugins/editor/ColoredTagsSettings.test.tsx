// @vitest-environment happy-dom
import { act } from 'preact/test-utils';
import { EditorView } from '@codemirror/view';
import { afterEach, expect, it } from 'vitest';
import { ColoredTagsSettings } from './ColoredTagsSettings';
import { DEFAULT_PLUGIN_SETTINGS } from '../../modules/settings';
import { legacyConfig } from '../testFixtures';
import { mountDom, actAndSettle, type MountedDom } from '../../testing/mountDom';
import { t } from '../../i18n';

let mounted: MountedDom;
afterEach(() => { act(() => mounted?.unmount()); });
it('pins and resets a nested tag without changing other plugin settings', async () => {
  let config = { ...legacyConfig(), plugins: structuredClone(DEFAULT_PLUGIN_SETTINGS) };
  const render = () => <ColoredTagsSettings config={config} workspacePath={null} onChange={next => { config = next; mounted.update(render()); }} />;
  await actAndSettle(() => { mounted = mountDom(render()); });
  const add = [...mounted.container.querySelectorAll<HTMLButtonElement>('button')].find(button => button.textContent === t('plugins.coloredTags.add'))!;
  expect(add.disabled).toBe(true);
  const input = mounted.container.querySelector<HTMLElement>(`[aria-label="${t('plugins.coloredTags.tag')}"]`)!;
  const view = EditorView.findFromDOM(input)!;
  await actAndSettle(() => { view.dispatch({ changes: { from: 0, insert: '#project/alpha' } }); });
  expect(add.disabled).toBe(false);
  await actAndSettle(() => add.click());
  expect(config.plugins.coloredTags.tagColors).toEqual({ 'project/alpha': 'blue' });
  expect(config.plugins.cursorTrail).toEqual(DEFAULT_PLUGIN_SETTINGS.cursorTrail);
  expect(config.plugins.coloredTags.mixNested).toBe(true);
  const remove = mounted.container.querySelector<HTMLButtonElement>(`[aria-label="${t('plugins.coloredTags.remove')} #project/alpha"]`)!;
  await actAndSettle(() => remove.click());
  expect(config.plugins.coloredTags.tagColors).toEqual({});
  await actAndSettle(() => mounted.container.querySelector<HTMLButtonElement>(`[aria-label="${t('plugins.coloredTags.mixNested')}"]`)!.click());
  const off = [...document.querySelectorAll<HTMLButtonElement>('[role="option"]')].find(button => button.textContent?.includes(t('plugins.coloredTags.mixOff')))!;
  await actAndSettle(() => off.click());
  expect(config.plugins.coloredTags.mixNested).toBe(false);
  expect(config.plugins.coloredTags.tagColors).toEqual({});
});
