// @vitest-environment happy-dom
import { useEffect } from 'react';
import { act } from 'preact/test-utils';
import { afterEach, expect, it, vi } from 'vitest';
import { SettingsForm } from '../SettingsForm';
import { SettingsDialog } from '../SettingsDialog';
import { PluginsSection } from './PluginsSection';
import { SettingsProvider, useSettingsStore, DEFAULT_PLUGIN_SETTINGS, type AppConfig } from '../../../modules/settings';
import { PLUGINS, usePluginEnabled, pluginSettings, type PluginId } from '../../../plugins/registry';
import { legacyConfig } from '../../../plugins/testFixtures';
import { mountDom, actAndSettle, type MountedDom } from '../../../testing/mountDom';
import { t } from '../../../i18n';
import { settingsSections, SETTINGS_SECTION_ICONS } from '../constants';
import { usePluginVaultData } from '../../../plugins/vaultData';

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn().mockResolvedValue(() => {}) }));

let mounted: MountedDom | undefined;
afterEach(() => { act(() => mounted?.unmount()); mounted = undefined; vi.clearAllMocks(); document.documentElement.removeAttribute('style'); });

it('keeps the report through dialog config reload and saves imported values on the next change', async () => {
  let saved: AppConfig = { ...legacyConfig(), plugins: structuredClone(DEFAULT_PLUGIN_SETTINGS) };
  let finishReload!: (config: AppConfig) => void;
  let reads = 0;
  invoke.mockReset().mockImplementation(async (command, args) => {
    if (command === 'get_settings') {
      if (++reads === 2) return new Promise<AppConfig>(resolve => { finishReload = resolve; });
      return structuredClone(saved);
    }
    if (command === 'obsidian_detect') return true;
    if (command === 'plugin_vault_data_get') return { version: 1, colors: {}, icons: {}, recentIcons: [], filters: { hide: [], pin: [] } };
    if (command === 'obsidian_import') {
      saved.plugins.gitSync.enabled = true;
      saved.plugins.gitSync.commitMessage = 'Imported backup';
      saved.dailyNotes.template = 'Templates/Daily.md';
      return { imported: ['obsidian-git', 'daily-notes'], overwritten: [], skipped: [] };
    }
    if (command === 'update_settings') saved = structuredClone(args.newConfig);
  });
  function Harness() {
    const { loadConfig } = useSettingsStore();
    useEffect(() => { void loadConfig(); }, [loadConfig]);
    return <SettingsDialog open workspacePath="/dialog-import" homePage="" onHomePageChange={() => {}} onClose={() => {}} />;
  }
  localStorage.setItem('aquilum_settings_section', JSON.stringify('plugins'));
  try {
    await actAndSettle(() => { mounted = mountDom(<SettingsProvider><Harness /></SettingsProvider>); });
    const importRoot = mounted!.container.querySelector('.q-plugins__import')!;
    await actAndSettle(() => importRoot.querySelector<HTMLButtonElement>('button')!.click());
    expect(mounted!.container.querySelector('.q-plugins__import')).toBe(importRoot);
    await actAndSettle(() => finishReload(structuredClone(saved)));
    expect(mounted!.container.querySelector('[role="status"]')?.textContent).toContain('obsidian-git');
    expect(pluginSwitch('gitSync').getAttribute('aria-checked')).toBe('true');
    expect(pluginCard('gitSync').textContent).toContain('Imported backup');
    await actAndSettle(() => pluginSwitch('folderCounts').click());
    expect(saved.plugins.gitSync.enabled).toBe(true);
    expect(saved.plugins.gitSync.commitMessage).toBe('Imported backup');
    expect(saved.dailyNotes.template).toBe('Templates/Daily.md');
  } finally { localStorage.removeItem('aquilum_settings_section'); }
});

function pluginCard(id: PluginId): HTMLElement {
  const section = [...mounted!.container.querySelectorAll<HTMLElement>('.q-plugins > .q-settings-block')]
    .find(section => section.querySelector(':scope > .q-settings-block__header')?.textContent === t(`plugins.${id}.name`));
  expect(section, `карточка ${id}`).toBeDefined();
  return section!.querySelector<HTMLElement>(':scope > .q-settings-block__card')!;
}

function pluginSwitch(id: PluginId): HTMLButtonElement {
  const button = [...pluginCard(id).querySelectorAll<HTMLButtonElement>(':scope > .q-plugins__row [role="switch"]')]
    .find(button => button.getAttribute('aria-label') === t(`plugins.${id}.name`));
  expect(button, `переключатель ${id}`).toBeDefined();
  return button!;
}

function pluginSwitches(): HTMLButtonElement[] {
  return [...mounted!.container.querySelectorAll<HTMLButtonElement>('.q-plugins > .q-settings-block > .q-settings-block__card > .q-plugins__row [role="switch"]')];
}

it('renders all plugins in registry order and toggles only the chosen enabled flag', async () => {
  let config = { ...legacyConfig(), plugins: structuredClone(DEFAULT_PLUGIN_SETTINGS) };
  const before = structuredClone(config);
  const render = () => <SettingsForm config={config} section="plugins" workspacePath={null} homePage="" onHomePageChange={() => {}} onChange={next => { config = next; mounted!.update(render()); }} />;
  await actAndSettle(() => { mounted = mountDom(render()); });
  const switches = pluginSwitches;
  expect(switches().map(button => button.getAttribute('aria-label'))).toEqual(PLUGINS.map(({ id }) => t(`plugins.${id}.name`)));
  expect(switches()).toHaveLength(PLUGINS.length);
  for (const { id } of PLUGINS) {
    await actAndSettle(() => pluginSwitch(id).click());
    expect(config.plugins[id].enabled).toBe(!before.plugins[id].enabled);
    expect(pluginSettings(config, id)).toEqual({ ...before.plugins[id], enabled: !before.plugins[id].enabled });
  }
  expect(config.dailyNotes).toEqual(before.dailyNotes);
  expect(config.mcp).toEqual(before.mcp);
  expect(settingsSections().find(({ id }) => id === 'plugins')?.label).toBe(t('settings.nav.plugins'));
  expect(SETTINGS_SECTION_ICONS.plugins).toBeDefined();
});

it('persists a plugin switch and updates usePluginEnabled immediately', async () => {
  let saved: AppConfig = { ...legacyConfig(), plugins: structuredClone(DEFAULT_PLUGIN_SETTINGS) };
  invoke.mockImplementation(async (command, args) => {
    if (command === 'get_settings') return structuredClone(saved);
    if (command === 'update_settings') saved = structuredClone(args.newConfig);
  });
  function Harness() {
    const { config, loadConfig, updateConfig } = useSettingsStore();
    const enabled = usePluginEnabled('gitSync');
    useEffect(() => { void loadConfig(); }, [loadConfig]);
    return <><output>{String(enabled)}</output>{config && <PluginsSection config={config} onChange={next => { void updateConfig(next); }} onNavigate={() => {}} />}</>;
  }
  await actAndSettle(() => { mounted = mountDom(<SettingsProvider><Harness /></SettingsProvider>); });
  expect(mounted!.container.querySelector('output')!.textContent).toBe('false');
  await actAndSettle(() => pluginSwitch('gitSync').click());
  expect(mounted!.container.querySelector('output')!.textContent).toBe('true');
  expect(saved.plugins.gitSync.enabled).toBe(true);
  expect(invoke).toHaveBeenLastCalledWith('update_settings', { newConfig: saved });
});

it('opens existing daily note settings from Calendar and returns to plugins', async () => {
  const config = { ...legacyConfig(), plugins: structuredClone(DEFAULT_PLUGIN_SETTINGS) };
  await actAndSettle(() => { mounted = mountDom(<SettingsForm config={config} section="plugins" workspacePath={null} homePage="" onHomePageChange={() => {}} onChange={() => {}} />); });
  const click = (label: string) => [...mounted!.container.querySelectorAll<HTMLButtonElement>('button')].find(button => button.textContent === label)!.click();
  await actAndSettle(() => click(t('plugins.calendar.dailyNotes')));
  expect(mounted!.container.textContent).toContain(t('settings.dailyNotes.section'));
  expect(mounted!.container.querySelectorAll('[role="switch"]')).toHaveLength(2);
  await actAndSettle(() => click(t('plugins.back')));
  expect(pluginSwitches()).toHaveLength(PLUGINS.length);
});

it('disables import without .obsidian and shows the reason', async () => {
  invoke.mockReset().mockImplementation(async command => command === 'obsidian_detect' ? false : undefined);
  const config = { ...legacyConfig(), plugins: structuredClone(DEFAULT_PLUGIN_SETTINGS) };
  await actAndSettle(() => { mounted = mountDom(<SettingsProvider><PluginsSection config={config} workspacePath="/no-obsidian" onChange={() => {}} onNavigate={() => {}} /></SettingsProvider>); });
  const button = mounted!.container.querySelector<HTMLButtonElement>('.q-plugins__import button')!;
  expect(button.disabled).toBe(true);
  expect(button.title).toBe(t('plugins.import.noObsidian'));
  expect(invoke).toHaveBeenCalledWith('obsidian_detect', { workspacePath: '/no-obsidian' });
  expect(invoke).not.toHaveBeenCalledWith('obsidian_import', expect.anything());
});

it('imports, shows the report, and reloads settings and existing vault-data consumers', async () => {
  let config: AppConfig = { ...legacyConfig(), plugins: structuredClone(DEFAULT_PLUGIN_SETTINGS) };
  let imported = false;
  invoke.mockReset().mockImplementation(async command => {
    if (command === 'get_settings') return structuredClone(config);
    if (command === 'obsidian_detect') return true;
    if (command === 'plugin_vault_data_get') return { version: 1, colors: {}, icons: { Projects: imported ? 'gamepad-2' : 'folder' }, recentIcons: [], filters: { hide: [], pin: [] } };
    if (command === 'obsidian_import') {
      imported = true;
      config.plugins.gitSync.enabled = true;
      return { imported: ['obsidian-git'], overwritten: ['Projects'], skipped: [{ id: 'unknown', reason: 'unsupported_plugin' }] };
    }
  });
  function Harness() {
    const { config, loadConfig } = useSettingsStore();
    const { data } = usePluginVaultData('/import-vault');
    useEffect(() => { void loadConfig(); }, [loadConfig]);
    return <><output>{data?.icons.Projects}:{String(config?.plugins.gitSync.enabled)}</output>{config && <PluginsSection config={config} workspacePath="/import-vault" onChange={() => {}} onNavigate={() => {}} />}</>;
  }
  await actAndSettle(() => { mounted = mountDom(<SettingsProvider><Harness /></SettingsProvider>); });
  expect(mounted!.container.querySelector('output')!.textContent).toBe('folder:false');
  const vaultReads = () => invoke.mock.calls.filter(([command]) => command === 'plugin_vault_data_get').length;
  const readsBeforeImport = vaultReads();
  await actAndSettle(() => mounted!.container.querySelector<HTMLButtonElement>('.q-plugins__import button')!.click());
  expect(invoke).toHaveBeenCalledWith('obsidian_import', { workspacePath: '/import-vault' });
  expect(invoke.mock.calls.filter(([command]) => command === 'get_settings')).toHaveLength(2);
  expect(vaultReads()).toBeGreaterThan(readsBeforeImport);
  expect(mounted!.container.querySelector('output')!.textContent).toBe('gamepad-2:true');
  const report = mounted!.container.querySelector('.q-plugins__import-report')!.textContent;
  expect(report).toContain('obsidian-git');
  expect(report).toContain('Projects');
  expect(report).toContain(t('plugins.import.reasons.unsupported_plugin'));
});

it('holds the import button while pending and reports a failed command', async () => {
  let reject!: (error: unknown) => void;
  invoke.mockReset().mockImplementation(async command => {
    if (command === 'obsidian_detect') return true;
    if (command === 'obsidian_import') return new Promise((_resolve, fail) => { reject = fail; });
  });
  const config = { ...legacyConfig(), plugins: structuredClone(DEFAULT_PLUGIN_SETTINGS) };
  await actAndSettle(() => { mounted = mountDom(<SettingsProvider><PluginsSection config={config} workspacePath="/failed-import" onChange={() => {}} onNavigate={() => {}} /></SettingsProvider>); });
  await actAndSettle(() => mounted!.container.querySelector<HTMLButtonElement>('.q-plugins__import button')!.click());
  expect(mounted!.container.querySelector<HTMLButtonElement>('.q-plugins__import button')!.disabled).toBe(true);
  expect(mounted!.container.querySelector('.q-plugins__import [aria-busy="true"]')).not.toBeNull();
  await actAndSettle(() => reject({ code: 'io', details: { message: 'Import failed' } }));
  expect(mounted!.container.querySelector('.q-plugins__import [role="alert"]')!.textContent).toBe('Import failed');
  expect(mounted!.container.querySelector<HTMLButtonElement>('.q-plugins__import button')!.disabled).toBe(false);
});

it('changes only gitSync settings through its registered settings controls', async () => {
  let config = { ...legacyConfig(), plugins: structuredClone(DEFAULT_PLUGIN_SETTINGS) };
  const before = structuredClone(config);
  const render = () => <PluginsSection config={config} onChange={next => { config = next; mounted!.update(render()); }} onNavigate={() => {}} />;
  await actAndSettle(() => { mounted = mountDom(render()); });
  await actAndSettle(() => mounted!.container.querySelector<HTMLButtonElement>(`[aria-label="${t('plugins.gitSync.push')}"]`)!.click());
  expect(config.plugins.gitSync.push).toBe(false);
  expect({ ...config, plugins: { ...config.plugins, gitSync: before.plugins.gitSync } }).toEqual(before);
});

it('expands a registered settings component and passes the current config', async () => {
  const plugin = PLUGINS.find(({ id }) => id === 'folderCounts')!;
  const previous = plugin.settings;
  plugin.settings = ({ config }) => <output>{String(config.plugins.folderCounts.hideZero)}</output>;
  try {
    const config = { ...legacyConfig(), plugins: structuredClone(DEFAULT_PLUGIN_SETTINGS) };
    await actAndSettle(() => { mounted = mountDom(<PluginsSection config={config} onChange={() => {}} onNavigate={() => {}} />); });
    const details = pluginCard('folderCounts').querySelector<HTMLDetailsElement>(':scope > details')!;
    expect(details.open).toBe(false);
    expect(details.querySelector('summary')!.textContent).toBe(t('plugins.settings'));
    expect(details.querySelector('output')!.textContent).toBe('true');
  } finally { plugin.settings = previous; }
});
