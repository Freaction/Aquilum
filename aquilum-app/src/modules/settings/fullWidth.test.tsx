// @vitest-environment happy-dom
import { useEffect, useRef } from 'react';
import type { EditorView } from '@codemirror/view';
import { useEditorBodyMenu } from '../../components/Editor/hooks/useEditorBodyMenu';
import { Menu } from '../../components/Common/Menu';
import { afterEach, expect, it, vi } from 'vitest';
import { act } from 'preact/test-utils';
import { SettingsProvider, useSettingsStore, DEFAULT_READER_SETTINGS, type AppConfig } from './index';
import { EditorSection } from '../../components/Settings/sections/EditorSection';
import { mountDom, actAndSettle, type MountedDom } from '../../testing/mountDom';
import { t } from '../../i18n';
import { DEFAULT_DAILY_NOTES_SETTINGS } from '../dailyNotes';

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));

const legacyConfig: Omit<AppConfig, 'editor' | 'plugins'> & { editor: Omit<AppConfig['editor'], 'fullWidth' | 'fullWidthShortcut'> } = {
  analysis: { enableBm25f: true, enableAdamicAdar: true, enableWikixiv: true,
    bm25fParams: { k1: 1.2, k3: 8, bTitle: 0.75, bBody: 0.75, titleWeight: 2 } },
  mcp: { enabled: false, port: 8787, token: '', allowWrite: false },
  trash: { retentionDays: 30 }, history: { retentionDays: 0 },
  search: { candidatePoolSize: 200, maxQueryTerms: 20 },
  editor: { fontFamily: DEFAULT_READER_SETTINGS.fontFamily, fontWeight: 400, fontSizeBase: 18,
    saveDebounceMs: 1000, lineHeight: 1.8, maxWidthCh: 80, smartDashes: true,
    listCallouts: true, autoLinkTitle: true, linkSuggest: true, linkSuggestMinChars: 2, liveTabs: 3 },
  reader: DEFAULT_READER_SETTINGS,
  ui: { fontFamily: DEFAULT_READER_SETTINGS.fontFamily, fontWeight: 400, fontSizeBase: 14,
    theme: 'dark', language: 'ru', primaryColor: '#4a80ff' },
  templates: { folder: '' }, files: { folder: '' }, updates: { auto: false },
  dailyNotes: DEFAULT_DAILY_NOTES_SETTINGS,
};

function SettingsHarness() {
  const { config, loadConfig, updateConfig } = useSettingsStore();
  useEffect(() => { void loadConfig(); }, [loadConfig]);
  return config ? <EditorSection config={config} onChange={(next) => { void updateConfig(next); }} /> : null;
}

function MenuHarness() {
  const { loadConfig } = useSettingsStore();
  useEffect(() => { void loadConfig(); }, [loadConfig]);
  const view = useRef<EditorView | null>(null);
  const menu = useEditorBodyMenu(view, false);
  return <><div data-editor onContextMenu={menu.onContextMenu}>Editor</div><Menu open={menu.open} position={menu.position} items={menu.items} onClose={menu.close} /></>;
}

let mounted: MountedDom | undefined;
afterEach(() => {
  act(() => mounted?.unmount());
  mounted = undefined;
  document.documentElement.removeAttribute('style');
  vi.clearAllMocks();
});

it('persists both width modes from the editor context menu without changing the readable width', async () => {
  let saved = structuredClone(legacyConfig) as AppConfig;
  invoke.mockImplementation(async (command: string, args?: { newConfig: AppConfig }) => {
    if (command === 'get_settings') return structuredClone(saved);
    if (command === 'update_settings') saved = structuredClone(args!.newConfig);
  });
  const mount = () => { mounted = mountDom(<SettingsProvider><MenuHarness /></SettingsProvider>); };
  await actAndSettle(mount);
  const width = () => document.documentElement.style.getPropertyValue('--q-editor-max-width');
  const choose = async (label: string) => {
    await actAndSettle(() => { mounted!.container.querySelector('[data-editor]')!.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true })); });
    const item = Array.from(document.querySelectorAll<HTMLElement>('[role="menuitem"]')).find(item => item.textContent === label);
    expect(item).toBeDefined();
    await actAndSettle(() => item!.click());
  };
  await choose(t('editor.widthFull'));
  expect(width()).toBe('none');
  expect(saved.editor.fullWidth).toBe(true);
  expect(saved.editor.maxWidthCh).toBe(80);
  await actAndSettle(() => { mounted!.unmount(); mount(); });
  expect(width()).toBe('none');
  await choose(t('editor.widthReadable'));
  expect(width()).toBe('80ch');
  expect(saved.editor.fullWidth).toBe(false);
  expect(saved.editor.maxWidthCh).toBe(80);
  await actAndSettle(() => { mounted!.unmount(); mount(); });
  expect(width()).toBe('80ch');
});

it('expands a legacy config and restores its saved column width after toggling and reloading', async () => {
  let saved = structuredClone(legacyConfig) as AppConfig;
  invoke.mockImplementation(async (command: string, args?: { newConfig: AppConfig }) => {
    if (command === 'get_settings') return structuredClone(saved);
    if (command === 'update_settings') saved = structuredClone(args!.newConfig);
  });
  await actAndSettle(() => { mounted = mountDom(<SettingsProvider><SettingsHarness /></SettingsProvider>); });
  const width = () => document.documentElement.style.getPropertyValue('--q-editor-max-width');
  const toggle = () => mounted!.container.querySelector<HTMLButtonElement>(`[role="switch"][aria-label="${t('settings.editor.fullWidth')}"]`);
  expect(width()).toBe('80ch');
  expect(toggle()).not.toBeNull();
  await actAndSettle(() => toggle()!.click());
  expect(width()).toBe('none');
  expect(saved.editor.maxWidthCh).toBe(80);
  expect(mounted!.container.querySelector(`[aria-label="${t('settings.editor.maxWidthAria')}"]`)).toBeNull();
  await actAndSettle(() => {
    mounted!.unmount();
    mounted = mountDom(<SettingsProvider><SettingsHarness /></SettingsProvider>);
  });
  expect(width()).toBe('none');
  expect(toggle()!.getAttribute('aria-checked')).toBe('true');
  await actAndSettle(() => toggle()!.click());
  expect(width()).toBe('80ch');
  expect(saved.editor.maxWidthCh).toBe(80);
  expect(mounted!.container.querySelector<HTMLElement>(`[aria-label="${t('settings.editor.maxWidthAria')}"]`)!.textContent).toBe('80');
});
