import { beforeEach, describe, expect, it, vi } from 'vitest';
import {
  openSession,
  renameDocument,
  resolveDocument,
  resolveWorkspace,
  saveStateBatch,
} from './gateway';
import { WorkspaceSession } from './workspaceSession';
import { emptyTabPath, type SessionTab } from './types';

vi.mock('./gateway', () => ({
  loadDocumentView: vi.fn(),
  markDocumentMissing: vi.fn(),
  openSession: vi.fn(),
  renameDocument: vi.fn(),
  resolveDocument: vi.fn(),
  resolveWorkspace: vi.fn(),
  saveStateBatch: vi.fn(),
}));

const resolveWorkspaceMock = vi.mocked(resolveWorkspace);
const openSessionMock = vi.mocked(openSession);
const saveStateBatchMock = vi.mocked(saveStateBatch);
const resolveDocumentMock = vi.mocked(resolveDocument);
const renameDocumentMock = vi.mocked(renameDocument);

function tab(documentId: string): SessionTab {
  return {
    tabId: crypto.randomUUID(),
    documentId,
    kind: 'document',
    path: `C:\\notes\\${documentId}.md`,
  };
}

describe('WorkspaceSession', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    resolveWorkspaceMock.mockResolvedValue(crypto.randomUUID());
    openSessionMock.mockResolvedValue({ activeTabId: null, tabs: [], views: [], graphCamera: null });
    saveStateBatchMock.mockResolvedValue(true);
  });

  it('coalesces rapid tab snapshots while one write is running', async () => {
    let releaseFirst: (accepted: boolean) => void = () => {};
    saveStateBatchMock.mockImplementationOnce(() => new Promise((resolve) => {
      releaseFirst = resolve;
    }));
    const session = await WorkspaceSession.open('C:\\notes');
    const first = tab('first');
    const second = tab('second');
    const latest = tab('latest');

    session.queueTabsSnapshot([first], first.tabId);
    session.queueTabsSnapshot([second], second.tabId);
    session.queueTabsSnapshot([latest], latest.tabId);

    expect(saveStateBatchMock).toHaveBeenCalledTimes(1);
    releaseFirst(true);
    await vi.waitFor(() => expect(saveStateBatchMock).toHaveBeenCalledTimes(2));
    expect(saveStateBatchMock.mock.calls[1][0].session?.tabs?.[0].documentId).toBe('latest');
    expect(saveStateBatchMock.mock.calls[1][0].sequence).toBe(2);
  });

  it('updates only the active tab when the tab structure is unchanged', async () => {
    const session = await WorkspaceSession.open('C:\\notes');
    const first = tab('first');
    const second = tab('second');
    session.queueTabsSnapshot([first, second], first.tabId);
    await vi.waitFor(() => expect(saveStateBatchMock).toHaveBeenCalledTimes(1));
    session.queueActiveTab(second.tabId);
    await vi.waitFor(() => expect(saveStateBatchMock).toHaveBeenCalledTimes(2));
    expect(saveStateBatchMock.mock.calls[0][0].session?.tabs).not.toBeNull();
    expect(saveStateBatchMock.mock.calls[1][0].session?.tabs).toBeNull();
  });

  it('keeps active-only persistence constant for a large restored session', async () => {
    const tabs = Array.from({ length: 10_000 }, (_, position) => ({
      tabId: crypto.randomUUID(),
      documentId: crypto.randomUUID(),
      kind: 'document' as const,
      position,
      relativePath: `${position}.md`,
    }));
    openSessionMock.mockResolvedValue({ activeTabId: tabs[0].tabId, tabs, views: [], graphCamera: null });
    const session = await WorkspaceSession.open('C:\\notes');
    session.queueActiveTab(tabs[9_999].tabId);
    await vi.waitFor(() => expect(saveStateBatchMock).toHaveBeenCalledTimes(1));
    expect(saveStateBatchMock.mock.calls[0][0].session).toEqual({
      activeTabId: tabs[9_999].tabId,
      tabs: null,
    });
  });

  it('does not retry after disposal', async () => {
    vi.useFakeTimers();
    saveStateBatchMock.mockRejectedValue(new Error('offline'));
    const session = await WorkspaceSession.open('C:\\notes');
    const first = tab('first');
    session.queueTabsSnapshot([first], first.tabId);
    expect(saveStateBatchMock).toHaveBeenCalledTimes(1);
    session.dispose();
    await vi.advanceTimersByTimeAsync(1_000);
    expect(saveStateBatchMock).toHaveBeenCalledTimes(1);
    vi.useRealTimers();
  });

  it('resolves the old identity before tracking a rename', async () => {
    const documentId = crypto.randomUUID();
    resolveDocumentMock.mockResolvedValue(documentId);
    renameDocumentMock.mockResolvedValue(undefined);
    const session = await WorkspaceSession.open('C:\\notes');

    await expect(session.resolveDocument(
      'C:\\notes\\old.md',
      'C:\\notes\\new.md',
    )).resolves.toBe(documentId);

    expect(resolveDocumentMock).toHaveBeenCalledWith(session.workspaceId, 'old.md');
    expect(renameDocumentMock).toHaveBeenCalledWith(documentId, 'new.md');
    expect(resolveDocumentMock.mock.invocationCallOrder[0])
      .toBeLessThan(renameDocumentMock.mock.invocationCallOrder[0]);
  });

  it('restores document and empty tabs in their stored order', async () => {
    const documentId = crypto.randomUUID();
    const documentTabId = crypto.randomUUID();
    const emptyTabId = crypto.randomUUID();
    openSessionMock.mockResolvedValue({
      activeTabId: documentTabId,
      views: [],
      graphCamera: null,
      tabs: [
        {
          tabId: documentTabId,
          documentId,
          kind: 'document',
          position: 0,
          relativePath: 'folder/note.md',
        },
        {
          tabId: emptyTabId,
          documentId: null,
          kind: 'empty',
          position: 1,
          relativePath: null,
        },
      ],
    });
    const session = await WorkspaceSession.open('C:\\notes');
    expect(session.restoredTabs().map((value) => value.path)).toEqual([
      'C:\\notes\\folder\\note.md',
      emptyTabPath(emptyTabId),
    ]);
  });
});

describe('WorkspaceSession.restorable', () => {
  const graphTab: SessionTab = {
    tabId: crypto.randomUUID(),
    documentId: null,
    kind: 'graph',
    path: '__graph_tab__',
  };
  const emptyTab: SessionTab = {
    tabId: crypto.randomUUID(),
    documentId: null,
    kind: 'empty',
    path: emptyTabPath('1'),
  };

  it('keeps a document tab only while its file is there', () => {
    const present = tab('kept');
    const gone = tab('deleted');

    const restored = WorkspaceSession.restorable(
      [present, gone],
      (path) => path === present.path,
    );

    expect(restored).toEqual([present]);
  });

  it('keeps tabs that stand for no file at all', () => {
    const restored = WorkspaceSession.restorable([graphTab, emptyTab], () => false);

    expect(restored).toEqual([graphTab, emptyTab]);
  });
});

describe('WorkspaceSession graph camera', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    resolveWorkspaceMock.mockResolvedValue(crypto.randomUUID());
    saveStateBatchMock.mockResolvedValue(true);
  });

  it('hands back the camera the session was opened with', async () => {
    const camera = { centerX: 1.5, centerY: -2.5, scale: 30 };
    openSessionMock.mockResolvedValue({
      activeTabId: null,
      tabs: [],
      views: [],
      graphCamera: camera,
    });

    const session = await WorkspaceSession.open('C:\notes');

    expect(session.graphCamera()).toEqual(camera);
  });

  it('reports no camera for a workspace nobody looked at yet', async () => {
    openSessionMock.mockResolvedValue({
      activeTabId: null,
      tabs: [],
      views: [],
      graphCamera: null,
    });

    const session = await WorkspaceSession.open('C:\notes');

    expect(session.graphCamera()).toBeNull();
  });

  it('sends a moved camera through the same batch as the tabs', async () => {
    openSessionMock.mockResolvedValue({
      activeTabId: null,
      tabs: [],
      views: [],
      graphCamera: null,
    });
    const session = await WorkspaceSession.open('C:\notes');
    const camera = { centerX: 4, centerY: 8, scale: 120 };

    session.queueGraphCamera(camera);
    await vi.waitFor(() => expect(saveStateBatchMock).toHaveBeenCalled());

    expect(saveStateBatchMock.mock.calls[0][0]).toMatchObject({ graphCamera: camera });
    expect(session.graphCamera()).toEqual(camera);
  });

  it('forgets the camera once the session is disposed', async () => {
    openSessionMock.mockResolvedValue({
      activeTabId: null,
      tabs: [],
      views: [],
      graphCamera: null,
    });
    const session = await WorkspaceSession.open('C:\notes');

    session.dispose();
    session.queueGraphCamera({ centerX: 1, centerY: 1, scale: 1 });

    expect(saveStateBatchMock).not.toHaveBeenCalled();
  });
});
