// @vitest-environment happy-dom
import { act } from 'preact/test-utils';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { usePluginVaultData, type VaultPluginData } from './vaultData';
import { mountDom, actAndSettle, type MountedDom } from '../testing/mountDom';

const { invoke, listen } = vi.hoisted(() => ({ invoke: vi.fn(), listen: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));
vi.mock('@tauri-apps/api/event', () => ({ listen }));

const empty = (): VaultPluginData => ({ version: 1, colors: {}, icons: {}, recentIcons: [], filters: { hide: [], pin: [] } });
let current: ReturnType<typeof usePluginVaultData>;
function Harness({ path }: { path: string | null }) {
  current = usePluginVaultData(path);
  return <output>{JSON.stringify(current.data)}</output>;
}
let mounted: MountedDom | undefined;
let relocate: () => void;
let dispose: ReturnType<typeof vi.fn>;
beforeEach(() => {
  invoke.mockReset().mockResolvedValue(empty());
  dispose = vi.fn();
  listen.mockReset().mockImplementation(async (name, callback) => {
    expect(name).toBe('notes-relocated');
    relocate = () => callback({ payload: { moves: [], removed: [] } });
    return dispose;
  });
});
afterEach(() => { act(() => mounted?.unmount()); mounted = undefined; });

it('loads data and shares optimistic updates between consumers of one vault', async () => {
  await actAndSettle(() => { mounted = mountDom(<><Harness path="/load" /><Harness path="/load" /></>); });
  expect(invoke).toHaveBeenCalledWith('plugin_vault_data_get', { workspacePath: '/load' });
  let finish!: () => void;
  invoke.mockImplementation(() => new Promise<void>(resolve => { finish = resolve; }));
  let saved!: Promise<void>;
  await actAndSettle(() => { saved = current.update(data => { data.colors.A = 'blue'; }); });
  expect([...mounted!.container.querySelectorAll('output')].map(node => JSON.parse(node.textContent!).colors)).toEqual([{ A: 'blue' }, { A: 'blue' }]);
  expect(invoke).toHaveBeenLastCalledWith('plugin_vault_data_set', { workspacePath: '/load', data: { ...empty(), colors: { A: 'blue' } } });
  await actAndSettle(async () => { finish(); await saved; });
});

it('rolls back a rejected write and propagates the command error', async () => {
  await actAndSettle(() => { mounted = mountDom(<Harness path="/rollback" />); });
  const error = { code: 'io', details: { message: 'write failed' } };
  invoke.mockRejectedValue(error);
  await actAndSettle(async () => {
    await expect(current.update(data => { data.icons.A = 'folder'; })).rejects.toEqual(error);
  });
  expect(current.data).toEqual(empty());
});

it('reloads after relocation and switches vaults without accepting stale reads', async () => {
  let oldRead!: (value: VaultPluginData) => void;
  invoke.mockImplementation((_command, args) => args.workspacePath === '/old'
    ? new Promise(resolve => { oldRead = resolve; })
    : Promise.resolve({ ...empty(), icons: { B: 'book' } }));
  await actAndSettle(() => { mounted = mountDom(<Harness path="/old" />); });
  await actAndSettle(() => mounted!.update(<Harness path="/new" />));
  await actAndSettle(() => oldRead({ ...empty(), icons: { A: 'folder' } }));
  expect(current.data!.icons).toEqual({ B: 'book' });
  invoke.mockResolvedValue({ ...empty(), icons: { C: 'book' } });
  await actAndSettle(() => relocate());
  expect(current.data!.icons).toEqual({ C: 'book' });
  expect(invoke).toHaveBeenLastCalledWith('plugin_vault_data_get', { workspacePath: '/new' });
  await actAndSettle(() => mounted!.update(<Harness path={null} />));
  expect(current.data).toBeNull();
  expect(dispose).toHaveBeenCalled();
});

it('keeps later edits when an earlier queued write fails', async () => {
  await actAndSettle(() => { mounted = mountDom(<Harness path="/queued" />); });
  let reject!: (error: Error) => void;
  invoke.mockImplementationOnce(() => new Promise((_resolve, fail) => { reject = fail; })).mockResolvedValue(undefined);
  let first!: Promise<unknown>;
  let second!: Promise<void>;
  await actAndSettle(() => {
    first = current.update(data => { data.colors.A = 'red'; }).catch(error => error);
    second = current.update(data => { data.icons.B = 'book'; });
  });
  await actAndSettle(async () => { reject(new Error('failed')); await first; await second; });
  expect(current.data).toEqual({ ...empty(), icons: { B: 'book' } });
  expect(invoke).toHaveBeenLastCalledWith('plugin_vault_data_set', { workspacePath: '/queued', data: { ...empty(), icons: { B: 'book' } } });
});

it('does not drop a relocation refresh while the initial read is in flight', async () => {
  let finish!: (value: VaultPluginData) => void;
  invoke.mockImplementationOnce(() => new Promise(resolve => { finish = resolve; }))
    .mockResolvedValue({ ...empty(), colors: { renamed: 'blue' } });
  await actAndSettle(() => { mounted = mountDom(<Harness path="/busy-read" />); });
  await actAndSettle(() => relocate());
  await actAndSettle(() => finish({ ...empty(), colors: { original: 'blue' } }));
  expect(current.data!.colors).toEqual({ renamed: 'blue' });
});
