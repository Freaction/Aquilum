// @vitest-environment happy-dom
import { act } from 'preact/test-utils';
import { actAndSettle, mountDom, type MountedDom } from '../../testing/mountDom';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { useDocumentLinks, type DocumentLinks, type LinkMode } from './useDocumentLinks';

const gateway = vi.hoisted(() => ({
  getBacklinks: vi.fn(),
  getOutgoingLinks: vi.fn(),
}));

vi.mock('./gateway', () => gateway);

let latest: { result: DocumentLinks | null; loading: boolean } | null = null;

function Harness({ mode }: { mode: LinkMode }) {
  latest = useDocumentLinks(mode, 'C:\\notes', 'C:\\notes\\Current.md', true, 1, true);
  return null;
}

describe('useDocumentLinks', () => {
  let renderer: MountedDom | null = null;

  afterEach(() => {
    if (renderer) act(() => renderer?.unmount());
    renderer = null;
    latest = null;
    vi.clearAllMocks();
  });

  it('ignores a stale request after the mode changes', async () => {
    let resolveBacklinks!: (links: Array<{ path: string; title: string; offset: number }>) => void;
    gateway.getBacklinks.mockReturnValue(new Promise((resolve) => {
      resolveBacklinks = resolve;
    }));
    gateway.getOutgoingLinks.mockResolvedValue([
      { target: 'Target', title: 'Target', path: 'C:\\notes\\Target.md' },
    ]);

    await actAndSettle(() => {
      renderer = mountDom(<Harness mode="backlinks" />);
    });
    await actAndSettle(() => {
      renderer?.update(<Harness mode="outgoing" />);
    });

    expect(latest?.result?.mode).toBe('outgoing');

    await actAndSettle(() => {
      resolveBacklinks([{ path: 'C:\\notes\\Source.md', title: 'Source', offset: 1 }]);
    });

    expect(latest?.result?.mode).toBe('outgoing');
    expect(gateway.getBacklinks).toHaveBeenCalledOnce();
    expect(gateway.getOutgoingLinks).toHaveBeenCalledOnce();
  });
});
