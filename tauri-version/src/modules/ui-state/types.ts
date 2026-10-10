type TabKind = 'document' | 'empty' | 'graph';

export interface StateFailure {
  error: unknown;
  databaseBroken: boolean;
}

const BROKEN_DATABASE_CODES = new Set(['unavailable', 'unsupported_schema']);

export function stateFailure(error: unknown, sessionFailed: boolean): StateFailure {
  const code = typeof error === 'object' && error !== null && 'code' in error ? String(error.code) : '';
  return { error, databaseBroken: sessionFailed || BROKEN_DATABASE_CODES.has(code) };
}

export const GRAPH_TAB_PATH = '__graph_tab__';

const EMPTY_TAB_PREFIX = '__empty_tab__';

export function emptyTabPath(tabId: string): string {
  return `${EMPTY_TAB_PREFIX}${tabId}`;
}

export function isEmptyTabPath(path: string | null | undefined): boolean {
  return Boolean(path?.startsWith(EMPTY_TAB_PREFIX));
}

export interface StoredTab {
  tabId: string;
  documentId: string | null;
  kind: TabKind;
  position: number;
}

interface LoadedTab extends StoredTab {
  relativePath: string | null;
}

export interface ViewState {
  path?: string;
  documentId: string;
  paneId: string;
  cursorAnchor: number[];
  cursorHead: number[];
  fallbackAnchor: number;
  fallbackHead: number;
  scrollAnchor: number[];
  fallbackScrollAnchor: number;
  scrollOffsetPx: number;
  focusedSurface: string;
}

export interface GraphCameraState {
  centerX: number;
  centerY: number;
  scale: number;
}

export interface LoadedSession {
  activeTabId: string | null;
  tabs: LoadedTab[];
  views: ViewState[];
  graphCamera: GraphCameraState | null;
}

export interface OpenSessionInput {
  workspaceId: string;
  windowId: string;
  epoch: string;
  nowMs: number;
}

export interface SaveStateBatchInput {
  workspaceId: string;
  windowId: string;
  epoch: string;
  sequence: number;
  nowMs: number;
  session: {
    activeTabId: string | null;
    tabs: StoredTab[] | null;
  } | null;
  views: ViewState[];
  graphCamera: GraphCameraState | null;
}

export interface SessionTab {
  tabId: string;
  documentId: string | null;
  kind: TabKind;
  path: string;
  identityPath?: string;
  mountKey?: string;
}

export function viewKey(documentId: string, paneId: string): string {
  return `${documentId}:${paneId}`;
}
