// @vitest-environment happy-dom
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { actAndSettle, mountDom, type MountedDom } from '../../../testing/mountDom';
import { DailyNotesFolderPicker } from './DailyNotesFolderPicker';
const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));
let mounted: MountedDom;
let value = '';
let workspace = '/vault';
const folder = (id: string) => ({ id, name: id.split('/').pop(), type: 'folder' });
const tree: Record<string, unknown[]> = {
  '/vault': [folder('/vault/Journal'), folder('/vault/Archive'), folder('/vault/node_modules'), folder('/vault/.obsidian')],
  '/vault/Journal': [folder('/vault/Journal/Daily')], '/vault/Journal/Daily': [], '/vault/Archive': [],
};
beforeEach(() => {
  value = ''; workspace = '/vault'; invoke.mockReset();
  invoke.mockImplementation(async (_command, args) => tree[args.path] ?? []);
});
afterEach(() => mounted?.unmount());
function render() { return <DailyNotesFolderPicker workspacePath={workspace} value={value} onChange={next => { value = next; mounted.update(render()); }} />; }
async function mount() { await actAndSettle(() => { mounted = mountDom(render()); }); }
function input() { return mounted.container.querySelector<HTMLInputElement>('[role="combobox"]')!; }
async function type(text: string) { await actAndSettle(() => { input().value = text; input().dispatchEvent(new Event('input', { bubbles: true })); }); }
async function key(key: string) { await actAndSettle(() => { input().dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true })); }); }
it('loads all nested folders once, filters locally, and selects relative paths with keyboard', async () => {
  await mount();
  await actAndSettle(() => input().focus());
  expect(invoke).toHaveBeenCalledWith('read_directory', { path: '/vault/Journal/Daily' });
  expect(invoke.mock.calls.some(([,args]) => args.path.includes('node_modules') || args.path.includes('.obsidian'))).toBe(false);
  const reads = invoke.mock.calls.length;
  await type('daily');
  expect(value).toBe('');
  expect(mounted.container.querySelectorAll('[role="option"]')).toHaveLength(2); // root and nested match
  await key('ArrowDown'); await key('ArrowDown');
  expect(input().getAttribute('aria-activedescendant')).toBeTruthy();
  await key('Enter');
  expect(value).toBe('Journal/Daily');
  expect(input().getAttribute('aria-expanded')).toBe('false');
  await actAndSettle(() => { input().blur(); input().focus(); });
  await type('archive');
  expect(invoke.mock.calls).toHaveLength(reads);
  await key('Escape');
  expect(value).toBe('Journal/Daily');
  expect(input().value).toBe('Journal/Daily');
});
it('keeps available folders after a branch error, skips cycles, and selects by mouse without saving the search', async () => {
  invoke.mockImplementation(async (_command, args) => {
    if (args.path === '/vault/Archive') throw { code: 'io' };
    if (args.path === '/vault/Journal/Daily') return [folder('/vault/Journal')];
    return tree[args.path] ?? [];
  });
  await mount(); await actAndSettle(() => input().focus());
  expect(mounted.container.querySelector('[role="alert"]')).not.toBeNull();
  expect(invoke.mock.calls.filter(([,args]) => args.path === '/vault/Journal')).toHaveLength(1);
  await type('daily');
  const option = mounted.container.querySelectorAll<HTMLButtonElement>('[role="option"]')[1];
  await actAndSettle(() => {
    const down = new MouseEvent('mousedown', { bubbles: true, cancelable: true });
    option.dispatchEvent(down);
    expect(down.defaultPrevented).toBe(true);
    option.click();
  });
  expect(value).toBe('Journal/Daily');
  await actAndSettle(() => input().blur());
  expect(value).toBe('Journal/Daily');
});

it('reopens on a focused input click and lets keyboard focus reach retry', async () => {
  invoke.mockRejectedValue({ code: 'io' });
  await mount(); await actAndSettle(() => input().focus());
  const retry = mounted.container.querySelector<HTMLButtonElement>('[role="alert"] button')!;
  await actAndSettle(() => retry.focus());
  expect(input().getAttribute('aria-expanded')).toBe('true');
  invoke.mockImplementation(async (_command, args) => tree[args.path] ?? []);
  await actAndSettle(() => { retry.click(); input().focus(); });
  await key('ArrowDown'); await key('Enter');
  expect(input().getAttribute('aria-expanded')).toBe('false');
  await actAndSettle(() => input().click());
  expect(input().getAttribute('aria-expanded')).toBe('true');
  expect(mounted.container.textContent).toContain('Journal/Daily');
});

it('falls back to the draft when refresh removes the active option', async () => {
  invoke.mockImplementation(async (_command, args) => {
    if (args.path === '/vault/Archive') throw { code: 'io' };
    return tree[args.path] ?? [];
  });
  await mount(); await actAndSettle(() => input().focus());
  await key('ArrowDown'); await key('ArrowDown'); await key('ArrowDown'); await key('ArrowDown');
  invoke.mockResolvedValue([]);
  await actAndSettle(() => mounted.container.querySelector<HTMLButtonElement>('[role="alert"] button')!.click());
  await key('Enter');
  expect(value).toBe('');
});

it('supports a new path and selection of the workspace root', async () => {
  await mount(); await actAndSettle(() => input().focus());
  await type('New/Nested'); await key('Enter');
  expect(value).toBe('New/Nested');
  await actAndSettle(() => { input().blur(); input().focus(); });
  await actAndSettle(() => mounted.container.querySelector<HTMLButtonElement>('[role="option"]')!.click());
  expect(value).toBe('');
});
it('ignores stale reads after a workspace roundtrip and reports current errors with retry', async () => {
  let resolve!: (value: unknown[]) => void;
  invoke.mockImplementationOnce(() => new Promise(done => { resolve = done; }));
  await mount(); await actAndSettle(() => input().focus());
  workspace = '/second'; await actAndSettle(() => mounted.update(render()));
  workspace = '/vault'; await actAndSettle(() => mounted.update(render()));
  await actAndSettle(() => resolve([folder('/vault/Stale')]));
  expect(mounted.container.textContent).not.toContain('Stale');
  invoke.mockRejectedValue({ code: 'io' });
  await actAndSettle(() => { input().blur(); input().focus(); });
  expect(mounted.container.querySelector('[role="alert"]')).not.toBeNull();
  invoke.mockImplementation(async (_command, args) => tree[args.path] ?? []);
  await actAndSettle(() => mounted.container.querySelector<HTMLButtonElement>('[role="alert"] button')!.click());
  expect(mounted.container.querySelector('[role="alert"]')).toBeNull();
  expect(mounted.container.textContent).toContain('Journal/Daily');
});
