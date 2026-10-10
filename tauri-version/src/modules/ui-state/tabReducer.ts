import type { SessionTab } from './types';

export interface TabsState {
  tabs: SessionTab[];
  activeTabId: string;
}

export type TabsAction =
  | { type: 'reset'; tab: SessionTab }
  | { type: 'restore'; tabs: SessionTab[]; activeTabId: string }
  | { type: 'select'; path: string }
  | { type: 'open-file'; path: string; tabId: string }
  | { type: 'open-file-new-tab'; path: string; tabId: string }
  | { type: 'close'; path: string; fallback: SessionTab }
  | { type: 'new-tab'; tab: SessionTab }
  | { type: 'reorder'; from: number; to: number }
  | { type: 'open-graph'; tab: SessionTab }
  | { type: 'rename'; oldPath: string; newPath: string }
  | { type: 'resolve-identity'; tabId: string; path: string; documentId: string };

function reordered<T>(items: readonly T[], from: number, to: number): T[] {
  const next = [...items];
  const [moved] = next.splice(from, 1);
  if (moved === undefined) return [...items];
  next.splice(to, 0, moved);
  return next;
}

export function tabsReducer(state: TabsState, action: TabsAction): TabsState {
  switch (action.type) {
    case 'reset':
      return { tabs: [action.tab], activeTabId: action.tab.tabId };
    case 'restore':
      return { tabs: action.tabs, activeTabId: action.activeTabId };
    case 'select': {
      const selected = state.tabs.find((tab) => tab.path === action.path);
      return selected ? { ...state, activeTabId: selected.tabId } : state;
    }
    case 'open-file': {
      const existing = state.tabs.find((tab) => tab.path === action.path);
      if (existing) return { ...state, activeTabId: existing.tabId };
      const index = state.tabs.findIndex((tab) => tab.tabId === state.activeTabId);
      const current = index >= 0 ? state.tabs[index] : null;
      const replacement: SessionTab = {
        tabId: current?.tabId ?? action.tabId,
        documentId: null,
        kind: 'document',
        path: action.path,
        mountKey: action.tabId,
      };
      const tabs = [...state.tabs];
      if (index >= 0) tabs[index] = replacement;
      else tabs.push(replacement);
      return { tabs, activeTabId: replacement.tabId };
    }
    case 'open-file-new-tab': {
      const existing = state.tabs.find((tab) => tab.path === action.path);
      if (existing) return { ...state, activeTabId: existing.tabId };
      const tab: SessionTab = {
        tabId: action.tabId,
        documentId: null,
        kind: 'document',
        path: action.path,
      };
      return { tabs: [...state.tabs, tab], activeTabId: tab.tabId };
    }
    case 'close': {
      const index = state.tabs.findIndex((tab) => tab.path === action.path);
      if (index < 0) return state;
      const tabs = state.tabs.filter((tab) => tab.path !== action.path);
      if (tabs.length === 0) tabs.push(action.fallback);
      const activeTabId = state.tabs[index].tabId === state.activeTabId
        ? (tabs[index - 1]?.tabId ?? tabs[0].tabId)
        : state.activeTabId;
      return { tabs, activeTabId };
    }
    case 'new-tab':
      return { tabs: [...state.tabs, action.tab], activeTabId: action.tab.tabId };
    case 'reorder': {
      const { from, to } = action;
      const inside = (index: number) => index >= 0 && index < state.tabs.length;
      if (from === to || !inside(from) || !inside(to)) return state;
      return { ...state, tabs: reordered(state.tabs, from, to) };
    }
    case 'open-graph': {
      const existing = state.tabs.find((tab) => tab.kind === 'graph');
      if (existing) return { ...state, activeTabId: existing.tabId };
      return { tabs: [...state.tabs, action.tab], activeTabId: action.tab.tabId };
    }
    case 'rename':
      if (!state.tabs.some((tab) => tab.path === action.oldPath)) return state;
      return {
        ...state,
        tabs: state.tabs.map((tab) => tab.path === action.oldPath ? {
          ...tab,
          path: action.newPath,
          identityPath: tab.documentId ? undefined : (tab.identityPath ?? action.oldPath),
        } : tab),
      };
    case 'resolve-identity':
      return {
        ...state,
        tabs: state.tabs.map((tab) =>
          tab.tabId === action.tabId && tab.path === action.path
            ? { ...tab, documentId: action.documentId, identityPath: undefined }
            : tab),
      };
  }
}
