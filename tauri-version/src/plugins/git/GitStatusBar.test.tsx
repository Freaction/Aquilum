// @vitest-environment happy-dom
import { act } from 'preact/test-utils';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { GitStatusBar } from './GitStatusBar';
import { DEFAULT_PLUGIN_SETTINGS } from '../../modules/settings';
import { legacyConfig } from '../testFixtures';
import { mountDom, type MountedDom } from '../../testing/mountDom';
import { t } from '../../i18n';

const { invoke, listen, store } = vi.hoisted(() => ({ invoke: vi.fn(), listen: vi.fn(), store: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));
vi.mock('@tauri-apps/api/event', () => ({ listen }));
vi.mock('../../modules/settings', async original => ({ ...await original<object>(), useSettingsStore: store }));

const status = { branch: 'main', ahead: 2, behind: 1, changed: 3, conflicted: [] };
let config: ReturnType<typeof legacyConfig> & { plugins: typeof DEFAULT_PLUGIN_SETTINGS };
let mounted: MountedDom | undefined;
const settle = async (action = () => {}) => { await act(async () => { action(); await vi.advanceTimersByTimeAsync(0); }); };
beforeEach(() => {
  vi.useFakeTimers();
  config = { ...legacyConfig(), plugins: structuredClone(DEFAULT_PLUGIN_SETTINGS) };
  config.plugins.gitSync.enabled = true;
  store.mockReturnValue({ config });
  invoke.mockReset().mockResolvedValue(status);
  listen.mockReset().mockResolvedValue(vi.fn());
});
afterEach(() => { act(() => mounted?.unmount()); mounted = undefined; vi.useRealTimers(); vi.clearAllMocks(); });

it('shows branch and counts and holds a spinner while a manual sync is pending', async () => {
  await settle(() => { mounted = mountDom(<GitStatusBar workspacePath="/git-vault" />); });
  expect(mounted!.container.textContent).toContain('main');
  expect(mounted!.container.textContent).toContain('↑2 ↓1');
  let finish!: (value: typeof status) => void;
  invoke.mockImplementationOnce(() => new Promise(resolve => { finish = resolve; }));
  await settle(() => mounted!.container.querySelector<HTMLButtonElement>('button')!.click());
  expect(mounted!.container.querySelector('[aria-busy="true"]')).not.toBeNull();
  expect(mounted!.container.querySelector('.q-git-status__spinner')).not.toBeNull();
  expect(mounted!.container.querySelector('button')!.disabled).toBe(true);
  expect(invoke).toHaveBeenLastCalledWith('git_sync', { workspacePath: '/git-vault', now: expect.objectContaining({ year: expect.any(Number), month: expect.any(Number), second: expect.any(Number) }) });
  await settle(() => finish({ ...status, changed: 0 }));
  expect(mounted!.container.querySelector('[aria-busy="true"]')).toBeNull();
});

it('backs up on the configured interval only when changes exist and clears timers on unmount', async () => {
  config.plugins.gitSync.autoBackupMinutes = 1;
  await settle(() => { mounted = mountDom(<GitStatusBar workspacePath="/timer-vault" />); });
  await act(async () => { await vi.advanceTimersByTimeAsync(59_999); });
  expect(invoke.mock.calls.filter(([command]) => command === 'git_sync')).toHaveLength(0);
  await act(async () => { await vi.advanceTimersByTimeAsync(1); });
  expect(invoke.mock.calls.filter(([command]) => command === 'git_sync')).toHaveLength(1);
  invoke.mockResolvedValue({ ...status, changed: 0 });
  await act(async () => { await vi.advanceTimersByTimeAsync(60_000); });
  expect(invoke.mock.calls.filter(([command]) => command === 'git_sync')).toHaveLength(1);
  await settle(() => mounted!.unmount());
  const calls = invoke.mock.calls.length;
  await act(async () => { await vi.advanceTimersByTimeAsync(60_000); });
  expect(invoke).toHaveBeenCalledTimes(calls);
  mounted = undefined;
});

it('pulls once on open, switches vaults, and renders conflict files without console errors', async () => {
  config.plugins.gitSync.pullOnOpen = true;
  await settle(() => { mounted = mountDom(<GitStatusBar workspacePath="/open-vault" />); });
  expect(invoke).toHaveBeenCalledWith('git_pull', { workspacePath: '/open-vault' });
  await settle(() => mounted!.update(<GitStatusBar workspacePath="/open-vault" />));
  expect(invoke.mock.calls.filter(([command]) => command === 'git_pull')).toHaveLength(1);
  invoke.mockRejectedValueOnce({ code: 'conflict', details: { message: 'Merge stopped', files: ['note.md'] } });
  await settle(() => mounted!.update(<GitStatusBar workspacePath="/next-vault" />));
  expect(mounted!.container.querySelector('[role="alert"]')!.textContent).toContain('note.md');
  expect(mounted!.container.textContent).toContain('Merge stopped');
});

it('shows unavailable for missing git and does not run operations when disabled or no vault is open', async () => {
  invoke.mockRejectedValue({ code: 'git_not_found', details: { message: 'git unavailable' } });
  await settle(() => { mounted = mountDom(<GitStatusBar workspacePath="/missing-git" />); });
  expect(mounted!.container.textContent).toContain(t('plugins.gitSync.unavailable'));
  config.plugins.gitSync.enabled = false;
  await settle(() => mounted!.update(<GitStatusBar workspacePath="/missing-git" />));
  expect(mounted!.container.textContent).toBe('');
  config.plugins.gitSync.enabled = true;
  await settle(() => mounted!.update(<GitStatusBar workspacePath={null} />));
  expect(mounted!.container.textContent).toBe('');
  expect(invoke).toHaveBeenCalledTimes(1);
});

it('does not overlap timer work with a pending sync and ignores results from the previous vault', async () => {
  config.plugins.gitSync.autoBackupMinutes = 1;
  await settle(() => { mounted = mountDom(<GitStatusBar workspacePath="/old-vault" />); });
  let finish!: (value: typeof status) => void;
  invoke.mockImplementationOnce(() => new Promise(resolve => { finish = resolve; }));
  await settle(() => mounted!.container.querySelector<HTMLButtonElement>('button')!.click());
  const calls = invoke.mock.calls.length;
  await act(async () => { await vi.advanceTimersByTimeAsync(60_000); });
  expect(invoke).toHaveBeenCalledTimes(calls);
  await settle(() => mounted!.update(<GitStatusBar workspacePath="/new-vault" />));
  await settle(() => finish({ ...status, branch: 'old-result' }));
  expect(mounted!.container.textContent).toContain('main');
  expect(mounted!.container.textContent).not.toContain('old-result');
});

it('refreshes a file change received while an older status request is pending', async () => {
  let onChange!: (message: { payload: string[] }) => void;
  listen.mockImplementation(async (_name, callback) => { onChange = callback; return () => {}; });
  let finish!: (value: typeof status) => void;
  invoke.mockImplementationOnce(() => new Promise(resolve => { finish = resolve; }));
  await settle(() => { mounted = mountDom(<GitStatusBar workspacePath="/pending-status" />); });
  await act(async () => { onChange({ payload: ['note.md'] }); await vi.advanceTimersByTimeAsync(150); });
  expect(invoke).toHaveBeenCalledTimes(1);
  await settle(() => finish({ ...status, changed: 0 }));
  expect(invoke).toHaveBeenCalledTimes(2);
  expect(mounted!.container.querySelector('.q-git-status__counts')!.getAttribute('aria-label')).toBe(t('plugins.gitSync.status', { ahead: status.ahead, behind: status.behind, changed: status.changed }));
});
