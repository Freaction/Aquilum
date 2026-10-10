import {
  openSession,
  loadDocumentView,
  markDocumentMissing,
  renameDocument,
  resolveDocument,
  resolveWorkspace,
} from './gateway';
import { StateBatchQueue } from './stateBatchQueue';
import { getWindowId } from '../windowId';
import { absolutePath, relativePath } from '../paths';
import { emptyTabPath, GRAPH_TAB_PATH, viewKey } from './types';
import type {
  GraphCameraState,
  LoadedSession,
  ViewState,
  SessionTab,
  StoredTab,
} from './types';

function uuid(): string {
  return crypto.randomUUID();
}

export class WorkspaceSession {
  private graphCameraState: GraphCameraState | null = null;
  private views = new Map<string, ViewState | null>();
  private viewLoads = new Map<string, Promise<ViewState | null>>();
  private disabled = false;
  private readonly queue: StateBatchQueue;

  private constructor(
    readonly workspacePath: string,
    readonly workspaceId: string,
    readonly epoch: string,
    readonly loaded: LoadedSession,
  ) {
    this.queue = new StateBatchQueue(workspaceId, getWindowId(), epoch);
    this.graphCameraState = loaded.graphCamera ?? null;
    for (const view of loaded.views) {
      this.views.set(viewKey(view.documentId, view.paneId), view);
    }
  }

  static async open(workspacePath: string): Promise<WorkspaceSession> {
    const workspaceId = await resolveWorkspace(workspacePath, Date.now());
    const epoch = uuid();
    const loaded = await openSession({
      workspaceId,
      windowId: getWindowId(),
      epoch,
      nowMs: Date.now(),
    });
    return new WorkspaceSession(workspacePath, workspaceId, epoch, loaded);
  }

  static restorable(tabs: SessionTab[], exists: (path: string) => boolean): SessionTab[] {
    return tabs.filter((tab) => tab.kind !== 'document' || exists(tab.path));
  }

  restoredTabs(): SessionTab[] {
    return this.loaded.tabs.flatMap((tab) => {
      if (tab.kind === 'empty') {
        return [{ ...tab, path: emptyTabPath(tab.tabId) }];
      }
      if (tab.kind === 'graph') {
        return [{ ...tab, path: GRAPH_TAB_PATH }];
      }
      if (!tab.relativePath || !tab.documentId) return [];
      return [{ ...tab, path: absolutePath(this.workspacePath, tab.relativePath) }];
    });
  }

  async resolveDocument(filePath: string, finalPath = filePath): Promise<string> {
    const documentId = await resolveDocument(
      this.workspaceId,
      relativePath(this.workspacePath, filePath),
    );
    if (filePath !== finalPath) await this.renameDocument(documentId, finalPath);
    return documentId;
  }

  renameDocument(documentId: string, filePath: string): Promise<void> {
    return renameDocument(documentId, relativePath(this.workspacePath, filePath));
  }

  markDocumentMissing(documentId: string): Promise<void> {
    return markDocumentMissing(documentId, Date.now());
  }

  queueTabsSnapshot(tabs: SessionTab[], activeTabId: string | null): void {
    if (this.disabled) return;
    const storedTabs = tabs.map<StoredTab>((tab, position) => ({
      tabId: tab.tabId,
      documentId: tab.documentId,
      kind: tab.kind,
      position,
    }));
    this.queue.queueSession({
      activeTabId,
      tabs: storedTabs,
    });
  }

  queueActiveTab(activeTabId: string | null): void {
    if (this.disabled) return;
    this.queue.queueSession({
      activeTabId,
      tabs: null,
    });
  }

  loadedView(documentId: string, paneId = 'main'): ViewState | null {
    return this.views.get(viewKey(documentId, paneId)) ?? null;
  }

  isViewLoaded(documentId: string, paneId = 'main'): boolean {
    return this.views.has(viewKey(documentId, paneId));
  }

  ensureViewLoaded(documentId: string, paneId = 'main'): Promise<ViewState | null> {
    const key = viewKey(documentId, paneId);
    if (this.views.has(key)) return Promise.resolve(this.views.get(key) ?? null);
    const current = this.viewLoads.get(key);
    if (current) return current;
    const operation = loadDocumentView(
      this.workspaceId,
      getWindowId(),
      documentId,
      paneId,
    ).then((view) => {
      if (this.disabled) return null;
      this.views.set(key, view);
      return view;
    }).finally(() => this.viewLoads.delete(key));
    this.viewLoads.set(key, operation);
    return operation;
  }

  graphCamera(): GraphCameraState | null {
    return this.graphCameraState;
  }

  queueGraphCamera(camera: GraphCameraState): void {
    if (this.disabled) return;
    this.graphCameraState = camera;
    this.queue.queueGraphCamera(camera);
  }

  queueView(view: ViewState): void {
    if (this.disabled) return;
    this.queue.queueView(view);
    this.views.set(viewKey(view.documentId, view.paneId), view);
  }

  dispose(): void {
    this.disabled = true;
    this.queue.dispose();
    this.views.clear();
    this.viewLoads.clear();
  }
}
