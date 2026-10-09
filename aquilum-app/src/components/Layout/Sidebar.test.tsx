// @vitest-environment happy-dom
import { act } from 'preact/test-utils';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { mountDom, actAndSettle, type MountedDom } from '../../testing/mountDom';
import { DEFAULT_PLUGIN_SETTINGS } from '../../modules/settings';
import { legacyConfig } from '../../plugins/testFixtures';
import type { VaultPluginData } from '../../plugins/vaultData';
import { t } from '../../i18n';
import { Sidebar } from './Sidebar';

const state = vi.hoisted(() => ({ config: null as any, data: null as any, overview: null as any, update: vi.fn() }));
vi.mock('../../modules/settings', async importOriginal => ({ ...await importOriginal<typeof import('../../modules/settings')>(), useSettingsStore: () => ({ config: state.config }) }));
vi.mock('../../plugins/vaultData', () => ({ usePluginVaultData: () => ({ data: state.data, update: state.update }) }));
const opener = vi.hoisted(() => ({ revealItemInDir: vi.fn() }));
vi.mock('@tauri-apps/plugin-opener', () => opener);
vi.mock('../../plugins/explorer/useExplorerOverview', () => ({ useExplorerOverview: () => ({ overview: state.overview, error: null, loading: false }) }));
let mounted: MountedDom | undefined;
const folder = { id: '/vault/A', name: 'A', type: 'folder' as const };
const note = { id: '/vault/n.md', name: 'n', type: 'file' as const };
const empty = { id: '/vault/Empty', name: 'Empty', type: 'folder' as const };
const render = (onLoadDirectory: (path: string) => Promise<boolean> = async () => true) => <Sidebar activeFile={null} files={[folder, empty, note]} directories={new Map([['/vault', [folder, empty, note]]])} loadingDirectories={new Set()} workspacePath="/vault" workspaceName="Vault" onFileSelect={() => {}} onCreateNote={() => {}} onLoadDirectory={onLoadDirectory} isOpen onOpenSettings={() => {}} onOpenWorkspaces={() => {}} />;
const row = (path: string) => [...mounted!.container.querySelectorAll<HTMLElement>('[data-file-id]')].find(row => row.dataset.fileId === path)!;
const button = (label: string) => [...document.querySelectorAll<HTMLButtonElement>('button')].find(button => button.textContent === label || button.getAttribute('aria-label') === label)!;
async function menu(path: string) { await actAndSettle(() => { row(path).querySelector('button')!.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, clientX: 10, clientY: 10 })); }); }

async function showHidden() {
  const toggle = mounted!.container.querySelector<HTMLInputElement>('input[type="checkbox"]')!;
  await actAndSettle(() => { toggle.checked = true; toggle.dispatchEvent(new Event('change', { bubbles: true })); });
}
it('offers Hide then Unhide and removes only active compatible strict hide rules', async () => {
  await actAndSettle(() => { mounted = mountDom(render()); });
  await menu(note.id);
  await actAndSettle(() => button(t('plugins.explorerFilters.hide')).click());
  const strict = state.data.filters.hide[0];
  state.data.filters.hide = [strict, { ...strict, target: 'files' }, { ...strict, active: false }, { ...strict, target: 'folders' }];
  state.overview = { ...state.overview, hidden: ['n.md'] };
  await actAndSettle(() => mounted!.update(render()));
  await showHidden();
  await menu(note.id);
  expect(button(t('plugins.explorerFilters.hide'))).toBeUndefined();
  await actAndSettle(() => button(t('plugins.explorerFilters.unhide')).click());
  expect(state.data.filters.hide).toEqual([{ ...strict, active: false }, { ...strict, target: 'folders' }]);
});

it.each(['wildcard', 'regex', 'tag', 'parent'])('keeps hide rules and explains hiding by %s', async kind => {
  state.data.filters.hide = [{
    name: kind, active: true, kind: kind === 'tag' ? 'tag' : 'path', target: 'both',
    pattern: kind === 'parent' ? 'A' : kind === 'tag' ? 'hidden' : '*.md',
    patternType: kind === 'parent' || kind === 'tag' ? 'strict' : kind,
  }];
  state.overview = { ...state.overview, hidden: kind === 'parent' ? ['A'] : ['n.md'] };
  const target = kind === 'parent' ? { id: '/vault/A/child.md', name: 'child', type: 'file' as const } : note;
  const renderHidden = () => kind === 'parent' ? <Sidebar activeFile={null} files={[target]} directories={new Map([['/vault', [target]]])} loadingDirectories={new Set()} workspacePath="/vault" workspaceName="Vault" onFileSelect={() => {}} onCreateNote={() => {}} onLoadDirectory={async () => true} isOpen onOpenSettings={() => {}} onOpenWorkspaces={() => {}} /> : render();
  await actAndSettle(() => { mounted = mountDom(renderHidden()); });
  await showHidden();
  await menu(target.id);
  await actAndSettle(() => button(t('plugins.explorerFilters.unhide')).click());
  expect(state.update).not.toHaveBeenCalled();
  expect(state.data.filters.hide).toHaveLength(1);
  expect(mounted!.container.querySelector('[role="alert"]')?.textContent).toBe(t('plugins.explorerFilters.hiddenByFilter'));
});
beforeEach(() => {
  localStorage.clear();
  state.config = { ...legacyConfig(), plugins: structuredClone(DEFAULT_PLUGIN_SETTINGS) };
  for (const id of ['folderCounts', 'fileColors', 'fileIcons', 'explorerFilters'] as const) state.config.plugins[id].enabled = true;
  state.config.plugins.fileColors.cascade = true;
  state.data = { version: 1, colors: { A: 'blue' }, icons: { A: 'calendar-days' }, recentIcons: [], filters: { hide: [], pin: [] } } satisfies VaultPluginData;
  state.overview = { counts: { A: 3, Empty: 0 }, hidden: [], pinned: [], errors: [] };
  state.update.mockReset().mockImplementation(async (mutator: (data: VaultPluginData) => void) => { const draft = structuredClone(state.data); mutator(draft); state.data = draft; mounted?.update(render()); });
});
afterEach(() => { act(() => mounted?.unmount()); mounted = undefined; });

describe('explorer sidebar integration', () => {
  it.each(['pin', 'hide'] as const)('does not reuse a strict %s rule for an incompatible target', async list => {
    const incompatible = { name: 'folders only', active: false, kind: 'path', target: 'folders', pattern: 'n.md', patternType: 'strict' } as const;
    state.data.filters[list] = [incompatible];
    await actAndSettle(() => { mounted = mountDom(render()); });
    await menu(note.id);
    await actAndSettle(() => button(t(`plugins.explorerFilters.${list}`)).click());
    expect(state.data.filters[list]).toEqual([
      incompatible,
      { name: 'n.md', active: true, kind: 'path', target: 'both', pattern: 'n.md', patternType: 'strict' },
    ]);
    if (list === 'pin') {
      state.data.filters.pin[0].active = true;
      state.overview = { ...state.overview, pinned: ['n.md'] };
      await actAndSettle(() => mounted!.update(render()));
      await menu(note.id);
      await actAndSettle(() => button(t('plugins.explorerFilters.unpin')).click());
      expect(state.data.filters.pin).toEqual([{ ...incompatible, active: true }]);
    }
  });

  it('does not close a newly opened picker when an earlier save finishes', async () => {
    let resolveSave!: () => void;
    const saved = new Promise<void>(resolve => { resolveSave = resolve; });
    state.update.mockImplementationOnce(async (mutator: (data: VaultPluginData) => void) => {
      const draft = structuredClone(state.data); mutator(draft); state.data = draft;
      await saved;
    });
    await actAndSettle(() => { mounted = mountDom(render()); });
    await menu(folder.id);
    await actAndSettle(() => button(t('plugins.fileColors.choose')).click());
    await actAndSettle(() => button(t('plugins.fileColors.colors.red')).click());
    await actAndSettle(() => button(t('plugins.fileColors.close')).click());
    await menu(note.id);
    await actAndSettle(() => button(t('plugins.fileColors.choose')).click());
    const picker = document.querySelector('[role="dialog"]');
    expect(picker).not.toBeNull();
    await actAndSettle(() => resolveSave());
    expect(document.querySelector('[role="dialog"]')).toBe(picker);
    expect(state.data.colors.A).toBe('red');
    await actAndSettle(() => button(t('plugins.fileColors.colors.green')).click());
    expect(state.data.colors['n.md']).toBe('green');
  });

  it('loads a hidden expanded unloaded folder when hidden items are revealed', async () => {
    localStorage.setItem('aquilum_expanded_folders:/vault', JSON.stringify(['A']));
    state.overview = { ...state.overview, hidden: ['A'] };
    const load = vi.fn(async () => true);
    await actAndSettle(() => { mounted = mountDom(render(load)); });
    expect(row(folder.id)).toBeUndefined();
    expect(load).not.toHaveBeenCalled();
    const toggle = mounted!.container.querySelector<HTMLInputElement>('input[type="checkbox"]')!;
    await actAndSettle(() => { toggle.checked = true; toggle.dispatchEvent(new Event('change', { bubbles: true })); });
    expect(row(folder.id)).toBeDefined();
    expect(load).toHaveBeenCalledWith(folder.id);
  });
  it('renders counts and icons with the folder chevron and obeys enabled flags', async () => {
    await actAndSettle(() => { mounted = mountDom(render()); });
    expect(row(folder.id).querySelector('.q-file-count')?.textContent).toBe('3');
    expect(row(empty.id).querySelector('.q-file-count')).toBeNull();
    expect(row(folder.id).querySelectorAll('svg')).toHaveLength(2);
    expect(row(folder.id).style.getPropertyValue('--q-file-color')).toBe('var(--q-blue-500)');
    state.config = structuredClone(state.config);
    for (const id of ['folderCounts', 'fileColors', 'fileIcons', 'explorerFilters'] as const) state.config.plugins[id].enabled = false;
    await actAndSettle(() => mounted!.update(render()));
    expect(mounted!.container.querySelector('.q-file-count')).toBeNull();
    expect(mounted!.container.querySelector('.q-file-custom-icon')).toBeNull();
    expect(mounted!.container.querySelector('.q-file-colored')).toBeNull();
    await menu(note.id);
    expect(button(t('plugins.fileColors.choose'))).toBeUndefined();
    expect(button(t('plugins.fileIcons.choose'))).toBeUndefined();
    expect(button(t('plugins.explorerFilters.pin'))).toBeUndefined();
  });

  it('saves a selected color through the vault mutator and restores row focus', async () => {
    await actAndSettle(() => { mounted = mountDom(render()); });
    await menu(folder.id);
    await actAndSettle(() => button(t('plugins.fileColors.choose')).click());
    await actAndSettle(() => button(t('plugins.fileColors.colors.red')).click());
    expect(state.data.colors.A).toBe('red');
    expect(row(folder.id).style.getPropertyValue('--q-file-color')).toBe('var(--q-red-500)');
    expect(document.querySelector('[role="dialog"]')).toBeNull();
    expect(document.activeElement).toBe(row(folder.id).querySelector('button'));
  });

  it('adds and removes only strict pin rules and explains pins from other rules', async () => {
    await actAndSettle(() => { mounted = mountDom(render()); });
    await menu(note.id);
    await actAndSettle(() => button(t('plugins.explorerFilters.pin')).click());
    expect(state.data.filters.pin).toEqual([{ name: 'n.md', active: true, kind: 'path', target: 'both', pattern: 'n.md', patternType: 'strict' }]);
    state.overview = { ...state.overview, pinned: ['n.md'] };
    await actAndSettle(() => mounted!.update(render()));
    await menu(note.id);
    await actAndSettle(() => button(t('plugins.explorerFilters.unpin')).click());
    expect(state.data.filters.pin).toEqual([]);
    state.data = { ...state.data, filters: { hide: [], pin: [{ name: 'all', active: true, kind: 'path', target: 'files', pattern: '*.md', patternType: 'wildcard' }] } };
    await actAndSettle(() => mounted!.update(render()));
    await menu(note.id);
    await actAndSettle(() => button(t('plugins.explorerFilters.unpin')).click());
    expect(state.data.filters.pin).toHaveLength(1);
    expect(mounted!.container.querySelector('[role="alert"]')?.textContent).toBe(t('plugins.explorerFilters.pinnedByFilter'));
  });

  it('keeps a failed color save open with a visible error', async () => {
    await actAndSettle(() => { mounted = mountDom(render()); });
    state.update.mockRejectedValueOnce(new Error('failed'));
    await menu(folder.id);
    await actAndSettle(() => button(t('plugins.fileColors.choose')).click());
    await actAndSettle(() => button(t('plugins.fileColors.colors.red')).click());
    expect(state.data.colors.A).toBe('blue');
    expect(document.querySelector('[role="dialog"] [role="alert"]')?.textContent).toBe(t('plugins.explorerFilters.saveError'));
  });
});

it('copies the vault-relative and system paths and reveals the item in the file manager', async () => {
  const writeText = vi.fn().mockResolvedValue(undefined);
  Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true });
  opener.revealItemInDir.mockResolvedValue(undefined);
  await actAndSettle(() => { mounted = mountDom(render()); });
  const copy = async (label: string) => {
    await menu(note.id);
    const parent = button(t('fileTree.copyPath'));
    expect(parent.getAttribute('aria-haspopup')).toBe('menu');
    await actAndSettle(() => parent.click());
    expect(parent.getAttribute('aria-expanded')).toBe('true');
    expect(document.activeElement?.textContent).toBe(t('fileTree.copyPathFromVault'));
    await actAndSettle(() => button(label).click());
  };
  await copy(t('fileTree.copyPathFromVault'));
  expect(writeText).toHaveBeenLastCalledWith('n.md');
  expect(document.querySelector('[role="menu"]')).toBeNull();
  await copy(t('fileTree.copyPathAbsolute'));
  expect(writeText).toHaveBeenLastCalledWith('/vault/n.md');
  await menu(folder.id);
  const reveal = button(t('fileTree.revealInFinder')) ?? button(t('fileTree.revealInFileManager'));
  await actAndSettle(() => reveal.click());
  expect(opener.revealItemInDir).toHaveBeenCalledWith('/vault/A');
  writeText.mockRejectedValueOnce(new Error('denied'));
  await copy(t('fileTree.copyPathFromVault'));
  expect(mounted!.container.querySelector('[role="alert"]')!.textContent).toBe(t('fileTree.copyFailed'));
});
