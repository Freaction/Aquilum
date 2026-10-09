// @vitest-environment happy-dom
import { act } from 'preact/test-utils';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { mountDom, type MountedDom } from '../../testing/mountDom';
import { DEFAULT_PLUGIN_SETTINGS, type PluginSettings } from '../../modules/settings';
import { useExplorerOverview, type ExplorerOverview } from './useExplorerOverview';

const { invoke, handlers } = vi.hoisted(() => ({ invoke: vi.fn(), handlers: new Map<string, (payload: unknown) => void>() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));
vi.mock('../../hooks/useTauriEvent', () => ({ useTauriEvent: (name: string, handler: (payload: unknown) => void) => handlers.set(name, handler) }));
let tree: MountedDom | undefined;
let result: ReturnType<typeof useExplorerOverview>;
const sample: ExplorerOverview = { counts: { A: 2 }, hidden: ['hidden'], pinned: ['pin.md'], errors: [] };
const settings: PluginSettings = { ...DEFAULT_PLUGIN_SETTINGS, folderCounts: { ...DEFAULT_PLUGIN_SETTINGS.folderCounts, enabled: true }, explorerFilters: { enabled: true } };
function Probe({ root = '/vault', config = settings }: { root?: string; config?: PluginSettings }) {
  result = useExplorerOverview(root, null, config);
  return <span>{result.overview.counts.A}</span>;
}
async function advance(ms: number) { await act(async () => { await vi.advanceTimersByTimeAsync(ms); }); }
beforeEach(() => { vi.useFakeTimers(); invoke.mockReset().mockResolvedValue(sample); handlers.clear(); });
afterEach(() => { act(() => tree?.unmount()); tree = undefined; vi.useRealTimers(); });

describe('explorer overview updates', () => {
  it('debounces initial request and events for 150ms', async () => {
    act(() => { tree = mountDom(<Probe />); });
    await advance(149);
    expect(invoke).not.toHaveBeenCalled();
    await advance(1);
    expect(invoke).toHaveBeenCalledWith('explorer_overview', { workspacePath: '/vault' });
    expect(result.overview).toEqual(sample);
    act(() => { handlers.get('workspace-changed')!(['/vault/A/n.md']); handlers.get('links-changed')!({ workspacePath: '/vault' }); });
    await advance(149);
    expect(invoke).toHaveBeenCalledTimes(1);
    await advance(1);
    expect(invoke).toHaveBeenCalledTimes(2);
  });

  it('ignores a late response from a previous workspace', async () => {
    let resolve!: (value: ExplorerOverview) => void;
    invoke.mockImplementationOnce(() => new Promise<ExplorerOverview>(done => { resolve = done; }));
    act(() => { tree = mountDom(<Probe />); });
    await advance(150);
    act(() => tree!.update(<Probe root="/other" />));
    expect(result.overview.hidden).toEqual([]);
    await act(async () => { resolve(sample); });
    expect(result.overview.hidden).toEqual([]);
    await advance(150);
    expect(invoke).toHaveBeenLastCalledWith('explorer_overview', { workspacePath: '/other' });
    expect(result.overview.counts.A).toBe(2);
  });

  it('clears filters immediately when disabled and reports request errors', async () => {
    act(() => { tree = mountDom(<Probe />); });
    await advance(150);
    act(() => tree!.update(<Probe config={DEFAULT_PLUGIN_SETTINGS} />));
    expect(result.overview.hidden).toEqual([]);
    await advance(150);
    expect(invoke).toHaveBeenCalledTimes(1);
    invoke.mockRejectedValueOnce(new Error('failed'));
    act(() => tree!.update(<Probe />));
    await advance(150);
    expect(result.error).toBeTruthy();
  });
});
