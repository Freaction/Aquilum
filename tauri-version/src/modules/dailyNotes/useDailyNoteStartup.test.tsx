// @vitest-environment happy-dom
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { act } from 'preact/test-utils';
import { mountDom, actAndSettle, type MountedDom } from '../../testing/mountDom';
import { DEFAULT_DAILY_NOTES_SETTINGS as defaults, type DailyNotesSettings } from './index';
import { useDailyNoteStartup } from './useDailyNoteStartup';

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));
let mounted: MountedDom | undefined;
beforeEach(() => { invoke.mockReset(); });
afterEach(() => { act(() => mounted?.unmount()); mounted = undefined; });
const onOpen = vi.fn();
function Harness({ workspace = '/vault', ready = true, settings, open = onOpen }: { workspace?: string | null; ready?: boolean; settings?: DailyNotesSettings; open?: (path: string) => void }) {
  const error = useDailyNoteStartup(workspace, ready, settings, open);
  return <output>{error}</output>;
}

it('waits for settings and workspace readiness and opens once using the latest callback', async () => {
  onOpen.mockClear();
  let resolve!: (note: { path: string; created: boolean }) => void;
  invoke.mockReturnValue(new Promise<{ path: string; created: boolean }>(done => { resolve = done; }));
  await actAndSettle(() => { mounted = mountDom(<Harness ready={false} />); });
  expect(invoke).not.toHaveBeenCalled();
  await actAndSettle(() => mounted!.update(<Harness ready={false} settings={{ ...defaults, openOnStartup: true }} />));
  expect(invoke).not.toHaveBeenCalled();
  await actAndSettle(() => mounted!.update(<Harness settings={{ ...defaults, openOnStartup: true }} />));
  expect(invoke).toHaveBeenCalledTimes(1);
  expect(invoke).toHaveBeenCalledWith('calendar_open', expect.objectContaining({ workspacePath: '/vault', period: 'day', create: true }));
  const latest = vi.fn();
  await actAndSettle(() => mounted!.update(<Harness settings={{ ...defaults, openOnStartup: true }} open={latest} />));
  await actAndSettle(() => resolve({ path: '/vault/today.md', created: true }));
  expect(onOpen).not.toHaveBeenCalled();
  expect(latest).toHaveBeenCalledWith('/vault/today.md');
  expect(invoke).toHaveBeenCalledTimes(1);
});

it('does not open retroactively when enabled in an already loaded workspace', async () => {
  await actAndSettle(() => { mounted = mountDom(<Harness settings={defaults} />); });
  await actAndSettle(() => mounted!.update(<Harness settings={{ ...defaults, openOnStartup: true }} />));
  expect(invoke).not.toHaveBeenCalled();
});

it.each(['result', 'error'])('ignores a stale %s after changing workspaces', async outcome => {
  const settings = { ...defaults, openOnStartup: true };
  let reject!: (error: Error) => void;
  let resolve!: (note: { path: string; created: boolean }) => void;
  invoke.mockReturnValueOnce(new Promise<{ path: string; created: boolean }>((done, fail) => { resolve = done; reject = fail; })).mockResolvedValueOnce({ path: '/second/today.md', created: true });
  onOpen.mockClear();
  await actAndSettle(() => { mounted = mountDom(<Harness settings={settings} />); });
  await actAndSettle(() => mounted!.update(<Harness workspace='/second' settings={settings} />));
  await actAndSettle(() => { if (outcome === 'error') reject(new Error('old failure')); else resolve({ path: '/vault/old.md', created: true }); });
  expect(onOpen.mock.calls).toEqual([['/second/today.md']]);
  expect(mounted!.container.textContent).toBe('');
});

it('shows failures only for the current activation and clears them on workspace changes', async () => {
  invoke.mockRejectedValue(new Error('failed'));
  await actAndSettle(() => { mounted = mountDom(<Harness settings={{ ...defaults, openOnStartup: true }} />); });
  expect(mounted!.container.textContent).not.toBe('');
  await actAndSettle(() => mounted!.update(<Harness workspace='/second' settings={defaults} />));
  expect(mounted!.container.textContent).toBe('');
});

it('does not open an async result after unmounting', async () => {
  onOpen.mockClear();
  let resolve!: (note: { path: string; created: boolean }) => void;
  invoke.mockReturnValue(new Promise<{ path: string; created: boolean }>(done => { resolve = done; }));
  await actAndSettle(() => { mounted = mountDom(<Harness settings={{ ...defaults, openOnStartup: true }} />); });
  await actAndSettle(() => { mounted!.unmount(); mounted = undefined; resolve({ path: '/vault/today.md', created: true }); });
  expect(onOpen).not.toHaveBeenCalled();
});
