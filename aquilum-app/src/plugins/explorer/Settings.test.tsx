// @vitest-environment happy-dom
import { act } from 'preact/test-utils';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { mountDom, actAndSettle, type MountedDom } from '../../testing/mountDom';
import { legacyConfig } from '../testFixtures';
import { DEFAULT_PLUGIN_SETTINGS } from '../../modules/settings';
import { t } from '../../i18n';
import { FolderCountsSettings } from './FolderCountsSettings';
import { FileColorsSettings } from './FileColorsSettings';
import { ExplorerFiltersSettings } from './ExplorerFiltersSettings';
import type { VaultPluginData } from '../vaultData';
const state = vi.hoisted(() => ({ data: null as VaultPluginData | null, update: vi.fn(), errors: [] as { list: 'hide' | 'pin'; index: number; message: string }[] }));
vi.mock('../vaultData', () => ({ usePluginVaultData: () => ({ data: state.data, update: state.update }) }));
vi.mock('./useExplorerOverview', () => ({ useExplorerOverview: () => ({ overview: { counts: {}, hidden: [], pinned: [], errors: state.errors }, error: null, loading: false }) }));
vi.mock('../../components/Common/Input', () => ({ Input: ({ value, onChange, ariaLabel, disabled }: { value: string; onChange: (value: string) => void; ariaLabel: string; disabled?: boolean }) => <input aria-label={ariaLabel} value={value} disabled={disabled} onInput={event => onChange((event.target as HTMLInputElement).value)} /> }));
vi.mock('../../components/Common/Dropdown', () => ({ Dropdown: ({ value, options, onChange, ariaLabel, disabled }: { value: string; options: { value: string; label: string }[]; onChange: (value: string) => void; ariaLabel: string; disabled?: boolean }) => <select value={value} aria-label={ariaLabel} disabled={disabled} onChange={event => onChange((event.target as HTMLSelectElement).value)}>{options.map(option => <option value={option.value}>{option.label}</option>)}</select> }));
let mounted: MountedDom | undefined;
const config = () => ({ ...legacyConfig(), plugins: structuredClone(DEFAULT_PLUGIN_SETTINGS) });
const click = async (label: string) => actAndSettle(() => [...mounted!.container.querySelectorAll<HTMLButtonElement>('button')].find(button => button.textContent === label || button.getAttribute('aria-label') === label)!.click());
beforeEach(() => { state.data = { version: 1, colors: {}, icons: {}, recentIcons: [], filters: { hide: [{ name: 'Hidden', active: true, kind: 'path', target: 'both', pattern: 'secret', patternType: 'strict' }], pin: [] } }; state.errors = []; state.update.mockReset().mockImplementation(async mutate => { mutate(state.data); }); });
afterEach(() => { act(() => mounted?.unmount()); mounted = undefined; });
it('changes only the chosen folder count or color setting', async () => {
  for (const [Component, id, fields] of [[FolderCountsSettings, 'folderCounts', ['showAllFiles', 'hideZero']], [FileColorsSettings, 'fileColors', ['cascade', 'background']]] as const) {
    const initial = config(); const change = vi.fn();
    await actAndSettle(() => { mounted = mountDom(<Component config={initial} onChange={change} workspacePath={null} />); });
    for (const field of fields) {
      await click(t(`plugins.${id}.${field}`));
      expect(change.mock.lastCall![0]).toEqual({ ...initial, plugins: { ...initial.plugins, [id]: { ...initial.plugins[id], [field]: !initial.plugins[id][field as keyof typeof initial.plugins[typeof id]] } } });
    }
    act(() => mounted!.unmount()); mounted = undefined;
  }
});
it('keeps a new filter local until its pattern is complete, then saves it', async () => {
  await actAndSettle(() => { mounted = mountDom(<ExplorerFiltersSettings config={config()} onChange={() => {}} workspacePath="/vault" />); });
  await click(t('plugins.explorerFilters.addPin'));
  expect(state.update).not.toHaveBeenCalled();
  const input = mounted!.container.querySelector<HTMLInputElement>(`[aria-label="${t('plugins.explorerFilters.pattern')}"]`)!;
  await actAndSettle(() => { input.value = 'notes/*'; input.dispatchEvent(new Event('input', { bubbles: true })); });
  await click(t('plugins.explorerFilters.save'));
  expect(state.data!.filters.pin).toEqual([{ name: '', active: true, kind: 'path', target: 'both', pattern: 'notes/*', patternType: 'wildcard' }]);
});
it('toggles and removes filters immediately and displays overview errors', async () => {
  state.errors = [{ list: 'hide', index: 0, message: 'Invalid expression' }];
  await actAndSettle(() => { mounted = mountDom(<ExplorerFiltersSettings config={config()} onChange={() => {}} workspacePath="/vault" />); });
  expect(mounted!.container.textContent).toContain('Invalid expression');
  await click('Hidden'); expect(state.data!.filters.hide[0].active).toBe(false);
  await click(t('plugins.explorerFilters.remove')); expect(state.data!.filters.hide).toEqual([]);
});
it('displays failed saves and requires an open workspace', async () => {
  state.update.mockRejectedValue(new Error('write failed'));
  const props = { config: config(), onChange: () => {} };
  await actAndSettle(() => { mounted = mountDom(<ExplorerFiltersSettings {...props} workspacePath="/vault" />); });
  await click('Hidden'); expect(mounted!.container.textContent).toContain(t('plugins.explorerFilters.saveError'));
  await actAndSettle(() => mounted!.update(<ExplorerFiltersSettings {...props} workspacePath={null} />));
  expect(mounted!.container.textContent).toContain(t('plugins.explorerFilters.noWorkspace'));
  expect([...mounted!.container.querySelectorAll<HTMLButtonElement>('button')].every(button => button.disabled)).toBe(true);
});
it('edits every filter field locally and persists the completed rule', async () => {
  await actAndSettle(() => { mounted = mountDom(<ExplorerFiltersSettings config={config()} onChange={() => {}} workspacePath="/vault" />); });
  await click(t('plugins.explorerFilters.edit'));
  for (const [key, value, event] of [['filterName', 'Tag rule', 'input'], ['pattern', 'project/work', 'input'], ['kind', 'tag', 'change'], ['target', 'files', 'change'], ['patternType', 'regex', 'change']]) {
    await actAndSettle(() => {
      const element = mounted!.container.querySelector<HTMLInputElement | HTMLSelectElement>(`[aria-label="${t(`plugins.explorerFilters.${key}`)}"]`)!;
      element.value = value; element.dispatchEvent(new Event(event, { bubbles: true }));
    });
  }
  expect(state.update).not.toHaveBeenCalled();
  await click(t('plugins.explorerFilters.active'));
  await click(t('plugins.explorerFilters.save'));
  expect(state.data!.filters.hide[0]).toEqual({ name: 'Tag rule', active: false, kind: 'tag', target: 'files', pattern: 'project/work', patternType: 'regex' });
});
it('does not save invalid exact paths, and cancelling discards the local draft', async () => {
  await actAndSettle(() => { mounted = mountDom(<ExplorerFiltersSettings config={config()} onChange={() => {}} workspacePath="/vault" />); });
  await click(t('plugins.explorerFilters.edit'));
  for (const value of ['', '/secret', 'a//b', '../secret', 'a/./b', 'a\\b', 'C:secret', 'a\u0085b']) {
    await actAndSettle(() => {
      const element = mounted!.container.querySelector<HTMLInputElement>(`[aria-label="${t('plugins.explorerFilters.pattern')}"]`)!;
      element.value = value; element.dispatchEvent(new Event('input', { bubbles: true }));
    });
    const save = [...mounted!.container.querySelectorAll<HTMLButtonElement>('button')].find(button => button.textContent === t('plugins.explorerFilters.save'))!;
    expect(save.disabled, value).toBe(true);
  }
  await click(t('plugins.explorerFilters.cancel'));
  expect(state.update).not.toHaveBeenCalled();
  expect(state.data!.filters.hide[0].pattern).toBe('secret');
});
