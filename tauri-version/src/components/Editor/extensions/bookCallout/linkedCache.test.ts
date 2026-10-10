import { beforeEach, describe, expect, it, vi } from 'vitest';
import {
  bookCalloutSignature,
  prefetchLinkedCallouts,
  resolveCalloutData,
} from './linkedCache';

vi.mock('@tauri-apps/api/core', () => ({
  convertFileSrc: (path: string) => `asset://${path}`,
}));

vi.mock('../../../../modules/docs/noteFields', () => ({
  readNoteFields: vi.fn(),
}));

vi.mock('../../../../modules/docs/bookPageRuntime', () => ({
  getPages: vi.fn(() => null),
  hydrateFromFm: vi.fn((_absolute: string, fm: Record<string, unknown>) => ({
    title: 'My Book',
    author: String(fm?.author ?? ''),
    cover: String(fm?.cover ?? ''),
    bookFile: String(fm?.book_file ?? ''),
    pages: null,
  })),
}));

const { readNoteFields } = await import('../../../../modules/docs/noteFields');

const MODEL = {
  title: 'My Book',
  wikiTarget: 'My Book',
  author: '',
  cover: '',
  filePath: '',
};

function resolver(paths: (string | null)[]) {
  return vi.fn(async () => ({ paths, complete: true }));
}

let workspaceRun = 0;
let workspace = '';

describe('bookCallout linkedCache', () => {
  beforeEach(() => {
    workspaceRun += 1;
    workspace = `/workspace-${workspaceRun}`;
    vi.mocked(readNoteFields).mockReset();
    vi.mocked(readNoteFields).mockResolvedValue([]);
  });

  it('has nothing to draw for a wiki-linked row until its data arrives', () => {
    expect(resolveCalloutData(MODEL, workspace)).toBeNull();
  });

  it('draws a wiki-linked row at once when the markdown carries its own cover', () => {
    const data = resolveCalloutData({ ...MODEL, cover: 'Files/inline.jpg' }, workspace);
    expect(data?.coverUrl).toBe(`asset://${workspace}/Files/inline.jpg`);
    expect(data?.linked).toBe(true);
  });

  it('asks for every linked row of the document in a single pair of calls', async () => {
    const second = { ...MODEL, title: 'Other', wikiTarget: 'Other' };
    vi.mocked(readNoteFields).mockResolvedValue([
      { cover: 'Files/a.jpg' },
      { cover: 'Files/b.jpg' },
    ]);
    const resolveWikiLinks = resolver(['Books/My Book.md', 'Books/Other.md']);

    const result = await prefetchLinkedCallouts([MODEL, second, MODEL], workspace, resolveWikiLinks);
    expect(result.changed).toBe(true);

    expect(resolveWikiLinks).toHaveBeenCalledTimes(1);
    expect(resolveWikiLinks).toHaveBeenCalledWith(['My Book', 'Other']);
    expect(readNoteFields).toHaveBeenCalledTimes(1);
    expect(readNoteFields).toHaveBeenCalledWith(workspace, [
      `${workspace}/Books/My Book.md`,
      `${workspace}/Books/Other.md`,
    ]);
    expect(resolveCalloutData(MODEL, workspace)?.coverUrl)
      .toBe(`asset://${workspace}/Files/a.jpg`);
    expect(resolveCalloutData(second, workspace)?.coverUrl)
      .toBe(`asset://${workspace}/Files/b.jpg`);
  });

  it('reports a change so widgets know to redraw', async () => {
    vi.mocked(readNoteFields).mockResolvedValue([{ cover: 'Files/a.jpg' }]);
    const result = await prefetchLinkedCallouts([MODEL], workspace, resolver(['Books/My Book.md']));
    expect(result.changed).toBe(true);
  });

  it('signs a row by what it shows, so a new cover means a redraw', async () => {
    vi.mocked(readNoteFields).mockResolvedValue([{ cover: 'Files/a.jpg' }]);
    await prefetchLinkedCallouts([MODEL], workspace, resolver(['Books/My Book.md']));
    const first = bookCalloutSignature(resolveCalloutData(MODEL, workspace));

    vi.mocked(readNoteFields).mockResolvedValue([{ cover: 'Files/b.jpg' }]);
    await prefetchLinkedCallouts([MODEL], workspace, resolver(['Books/My Book.md']), true);
    const second = bookCalloutSignature(resolveCalloutData(MODEL, workspace));

    expect(second).not.toBe(first);
    expect(bookCalloutSignature(null)).toBe('');
  });

  it('redraws nothing when a refresh pass finds the same data', async () => {
    vi.mocked(readNoteFields).mockResolvedValue([{ cover: 'Files/a.jpg' }]);
    await prefetchLinkedCallouts([MODEL], workspace, resolver(['Books/My Book.md']));

    const again = await prefetchLinkedCallouts(
      [MODEL],
      workspace,
      resolver(['Books/My Book.md']),
      true,
    );
    expect(again.changed).toBe(false);
  });

  it('picks up a cover changed outside the app on a refresh pass', async () => {
    vi.mocked(readNoteFields).mockResolvedValue([{ cover: 'Files/a.jpg' }]);
    await prefetchLinkedCallouts([MODEL], workspace, resolver(['Books/My Book.md']));

    vi.mocked(readNoteFields).mockResolvedValue([{ cover: 'Files/b.jpg' }]);
    const refreshed = await prefetchLinkedCallouts(
      [MODEL],
      workspace,
      resolver(['Books/My Book.md']),
      true,
    );
    expect(refreshed.changed).toBe(true);
    expect(resolveCalloutData(MODEL, workspace)?.coverUrl)
      .toBe(`asset://${workspace}/Files/b.jpg`);
  });

  it('skips a row already resolved but retries one whose link did not resolve', async () => {
    const unresolved = resolver([null]);
    await prefetchLinkedCallouts([MODEL], workspace, unresolved);
    expect(resolveCalloutData(MODEL, workspace)).not.toBeNull();

    const retry = resolver(['Books/My Book.md']);
    vi.mocked(readNoteFields).mockResolvedValue([{ cover: 'Files/a.jpg' }]);
    await prefetchLinkedCallouts([MODEL], workspace, retry);
    expect(retry).toHaveBeenCalledTimes(1);
    expect(resolveCalloutData(MODEL, workspace)?.coverUrl)
      .toBe(`asset://${workspace}/Files/a.jpg`);

    const again = resolver(['Books/My Book.md']);
    await prefetchLinkedCallouts([MODEL], workspace, again);
    expect(again).not.toHaveBeenCalled();
  });
});
