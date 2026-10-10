// @vitest-environment happy-dom
import { act } from 'preact/test-utils';
import { actAndSettle, mountDom, type MountedDom } from '../../testing/mountDom';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { useLinkIndex } from './useLinkIndex';
import type { SearchIndexStatus } from '../search';

const events = vi.hoisted(() => ({
  listen: vi.fn(),
  deliver: null as null | ((event: { payload: { workspacePath: string } }) => void),
}));

const search = vi.hoisted(() => ({
  getSearchIndexStatus: vi.fn(),
  prepareSearchIndex: vi.fn(),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: (_name: string, handler: (event: { payload: { workspacePath: string } }) => void) => {
    events.deliver = handler;
    return Promise.resolve(() => {
      events.deliver = null;
    });
  },
}));

vi.mock('../search', () => search);
vi.mock('../idle', () => ({ whenIdle: (run: () => void) => { run(); return () => {}; } }));

const WORKSPACE = 'C:\\notes';

function status(generation: number, revision: number): SearchIndexStatus {
  return {
    state: 'ready',
    updating: false,
    generation,
    revision,
    indexedDocuments: 69,
    scannedDocuments: 69,
  };
}

let latest: { ready: boolean; revision: number } | null = null;

function Harness() {
  const { ready, revision } = useLinkIndex(WORKSPACE);
  latest = { ready, revision };
  return null;
}

async function mount(): Promise<MountedDom> {
  let renderer!: MountedDom;
  await actAndSettle(() => {
    renderer = mountDom(<Harness />);
  });
  return renderer;
}

async function change(): Promise<void> {
  await actAndSettle(() => {
    events.deliver?.({ payload: { workspacePath: WORKSPACE } });
  });
}

describe('useLinkIndex', () => {
  let renderer: MountedDom | null = null;

  afterEach(() => {
    if (renderer) act(() => renderer?.unmount());
    renderer = null;
    latest = null;
    events.deliver = null;
    vi.clearAllMocks();
  });

  it('reports a change whose backend revision collides with the local counter', async () => {
    search.prepareSearchIndex.mockResolvedValue(undefined);
    search.getSearchIndexStatus.mockResolvedValue(status(2, 1));

    renderer = await mount();
    const settled = latest?.revision ?? 0;

    search.getSearchIndexStatus.mockResolvedValue(status(2, settled));
    await change();

    expect(latest?.ready).toBe(true);
    expect(latest?.revision).not.toBe(settled);
  });

  it('every further change moves the revision again', async () => {
    search.prepareSearchIndex.mockResolvedValue(undefined);
    search.getSearchIndexStatus.mockResolvedValue(status(2, 1));

    renderer = await mount();
    const seen = new Set<number>([latest?.revision ?? 0]);

    for (const revision of [2, 3, 4]) {
      search.getSearchIndexStatus.mockResolvedValue(status(2, revision));
      await change();
      expect(seen.has(latest?.revision ?? 0)).toBe(false);
      seen.add(latest?.revision ?? 0);
    }
  });

  it('a repeated report of the same index state does not refetch the graph', async () => {
    search.prepareSearchIndex.mockResolvedValue(undefined);
    search.getSearchIndexStatus.mockResolvedValue(status(2, 7));

    renderer = await mount();
    const settled = latest?.revision ?? 0;

    await change();
    await change();

    expect(latest?.revision).toBe(settled);
  });

  it('an index that is still building is not served as ready', async () => {
    search.prepareSearchIndex.mockResolvedValue(undefined);
    search.getSearchIndexStatus.mockResolvedValue({ ...status(2, 1), state: 'indexing' as const });

    renderer = await mount();

    expect(latest?.ready).toBe(false);
  });

  it('handles workspacePath with different casing and separators', async () => {
    search.prepareSearchIndex.mockResolvedValue(undefined);
    search.getSearchIndexStatus.mockResolvedValue(status(2, 1));

    renderer = await mount();
    const settled = latest?.revision ?? 0;

    search.getSearchIndexStatus.mockResolvedValue(status(2, 10));
    await actAndSettle(() => {
      events.deliver?.({ payload: { workspacePath: 'c:/notes/' } });
    });

    expect(latest?.revision).not.toBe(settled);
  });
});
