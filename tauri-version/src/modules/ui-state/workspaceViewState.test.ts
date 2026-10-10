import { beforeEach, describe, expect, it, vi } from 'vitest';
import {
  loadDocumentView,
  openSession,
  resolveWorkspace,
  saveStateBatch,
} from './gateway';
import type { ViewState } from './types';
import { WorkspaceSession } from './workspaceSession';

vi.mock('./gateway', () => ({
  loadDocumentView: vi.fn(),
  markDocumentMissing: vi.fn(),
  openSession: vi.fn(),
  renameDocument: vi.fn(),
  resolveDocument: vi.fn(),
  resolveWorkspace: vi.fn(),
  saveStateBatch: vi.fn(),
}));

const loadDocumentViewMock = vi.mocked(loadDocumentView);
const openSessionMock = vi.mocked(openSession);
const resolveWorkspaceMock = vi.mocked(resolveWorkspace);
const saveStateBatchMock = vi.mocked(saveStateBatch);

function view(documentId: string, position: number): ViewState {
  return {
    documentId,
    paneId: 'main',
    cursorAnchor: [1],
    cursorHead: [2],
    fallbackAnchor: position,
    fallbackHead: position,
    scrollAnchor: [3],
    fallbackScrollAnchor: position,
    scrollOffsetPx: 4,
    focusedSurface: 'body',
  };
}

describe('WorkspaceSession view state', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    resolveWorkspaceMock.mockResolvedValue(crypto.randomUUID());
    openSessionMock.mockResolvedValue({ activeTabId: null, tabs: [], views: [], graphCamera: null });
    saveStateBatchMock.mockResolvedValue(true);
  });

  it('coalesces newer view state while a write is in flight', async () => {
    let release: (accepted: boolean) => void = () => {};
    saveStateBatchMock.mockImplementationOnce(() => new Promise((resolve) => {
      release = resolve;
    }));
    const session = await WorkspaceSession.open('C:\\notes');
    const documentId = crypto.randomUUID();
    session.queueView(view(documentId, 1));
    session.queueView(view(documentId, 9));
    expect(saveStateBatchMock).toHaveBeenCalledTimes(1);
    release(true);
    await vi.waitFor(() => expect(saveStateBatchMock).toHaveBeenCalledTimes(2));
    expect(saveStateBatchMock.mock.calls[1][0].views[0].fallbackAnchor).toBe(9);
  });

  it('loads one closed-document view once and then serves the cache', async () => {
    const documentId = crypto.randomUUID();
    const stored = view(documentId, 7);
    loadDocumentViewMock.mockResolvedValue(stored);
    const session = await WorkspaceSession.open('C:\\notes');
    expect(session.isViewLoaded(documentId)).toBe(false);
    const first = session.ensureViewLoaded(documentId);
    const second = session.ensureViewLoaded(documentId);
    expect(session.isViewLoaded(documentId)).toBe(false);
    await expect(Promise.all([first, second])).resolves.toEqual([stored, stored]);
    expect(session.isViewLoaded(documentId)).toBe(true);
    await expect(session.ensureViewLoaded(documentId)).resolves.toEqual(stored);
    expect(loadDocumentViewMock).toHaveBeenCalledTimes(1);
  });
});
