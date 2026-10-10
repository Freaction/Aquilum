import { describe, expect, it } from 'vitest';
import { tabsReducer, type TabsState } from './tabReducer';
import { emptyTabPath, GRAPH_TAB_PATH, type SessionTab } from './types';

function empty(id: string): SessionTab {
  return { tabId: id, documentId: null, kind: 'empty', path: emptyTabPath(id) };
}

function graph(id: string): SessionTab {
  return { tabId: id, documentId: null, kind: 'graph', path: GRAPH_TAB_PATH };
}

describe('tabsReducer', () => {
  it('atomically selects and replaces tabs without mutating the previous state', () => {
    const initial: TabsState = { tabs: [empty('one')], activeTabId: 'one' };
    const next = tabsReducer(initial, { type: 'open-file', path: 'note.md', tabId: 'unused' });
    expect(next.tabs).toEqual([expect.objectContaining({
      tabId: 'one', documentId: null, kind: 'document', path: 'note.md',
    })]);
    expect(next.activeTabId).toBe('one');
    expect(initial.tabs[0].kind).toBe('empty');
  });

  it('opens the graph in its own tab and never opens a second one', () => {
    const initial: TabsState = { tabs: [empty('one')], activeTabId: 'one' };

    const opened = tabsReducer(initial, { type: 'open-graph', tab: graph('two') });
    const reopened = tabsReducer(
      { ...opened, activeTabId: 'one' },
      { type: 'open-graph', tab: graph('three') },
    );

    expect(opened.tabs.map((tab) => tab.kind)).toEqual(['empty', 'graph']);
    expect(opened.activeTabId).toBe('two');
    expect(reopened.tabs).toEqual(opened.tabs);
    expect(reopened.activeTabId).toBe('two');
  });

  it('closes the graph tab like any other', () => {
    const state: TabsState = { tabs: [empty('one'), graph('two')], activeTabId: 'two' };

    const next = tabsReducer(state, { type: 'close', path: GRAPH_TAB_PATH, fallback: empty('x') });

    expect(next.tabs.map((tab) => tab.tabId)).toEqual(['one']);
    expect(next.activeTabId).toBe('one');
  });

  it('is deterministic when React evaluates the same action twice', () => {
    const initial: TabsState = { tabs: [empty('one')], activeTabId: 'one' };
    const action = { type: 'new-tab', tab: empty('two') } as const;
    expect(tabsReducer(initial, action)).toEqual(tabsReducer(initial, action));
  });

  it('clears documentId when the active tab is reused for another file', () => {
    const state: TabsState = {
      tabs: [{ tabId: 'one', documentId: 'doc-a', kind: 'document', path: 'a.md' }],
      activeTabId: 'one',
    };
    const next = tabsReducer(state, { type: 'open-file', path: 'b.md', tabId: 'unused' });
    expect(next.tabs).toEqual([expect.objectContaining({
      tabId: 'one', documentId: null, kind: 'document', path: 'b.md',
    })]);
    expect(next.activeTabId).toBe('one');
  });

  it('keeps the mount key when a tab is renamed and renews it for another file', () => {
    const state: TabsState = {
      tabs: [{
        tabId: 'one', documentId: 'doc-a', kind: 'document', path: 'a.md', mountKey: 'mount-a',
      }],
      activeTabId: 'one',
    };
    const renamed = tabsReducer(state, { type: 'rename', oldPath: 'a.md', newPath: 'b.md' });
    expect(renamed.tabs[0].mountKey).toBe('mount-a');

    const reused = tabsReducer(state, { type: 'open-file', path: 'c.md', tabId: 'unused' });
    expect(reused.tabs[0].mountKey).not.toBe('mount-a');
  });

  it('opens a search result in a separate tab when requested', () => {
    const initial: TabsState = { tabs: [empty('one')], activeTabId: 'one' };
    const next = tabsReducer(initial, {
      type: 'open-file-new-tab',
      path: 'result.md',
      tabId: 'result-tab',
    });
    expect(next.tabs).toHaveLength(2);
    expect(next.tabs[1]).toEqual(expect.objectContaining({
      tabId: 'result-tab',
      path: 'result.md',
    }));
    expect(next.activeTabId).toBe('result-tab');
  });

  it('keeps the identity source when a pending document is renamed', () => {
    const state: TabsState = {
      tabs: [{ tabId: 'one', documentId: null, kind: 'document', path: 'old.md' }],
      activeTabId: 'one',
    };
    const next = tabsReducer(state, { type: 'rename', oldPath: 'old.md', newPath: 'new.md' });
    expect(next.tabs[0]).toEqual(expect.objectContaining({
      path: 'new.md',
      identityPath: 'old.md',
    }));
  });
  it('moves a tab to a new position and keeps the active one', () => {
    const state: TabsState = {
      tabs: [
        { tabId: 'one', documentId: null, kind: 'document', path: 'a.md' },
        { tabId: 'two', documentId: null, kind: 'document', path: 'b.md' },
        { tabId: 'three', documentId: null, kind: 'document', path: 'c.md' },
      ],
      activeTabId: 'two',
    };
    const next = tabsReducer(state, { type: 'reorder', from: 0, to: 2 });
    expect(next.tabs.map((tab) => tab.path)).toEqual(['b.md', 'c.md', 'a.md']);
    expect(next.activeTabId).toBe('two');
  });

  it('ignores a move that changes nothing or points outside', () => {
    const state: TabsState = {
      tabs: [{ tabId: 'one', documentId: null, kind: 'document', path: 'a.md' }],
      activeTabId: 'one',
    };
    expect(tabsReducer(state, { type: 'reorder', from: 0, to: 0 })).toBe(state);
    expect(tabsReducer(state, { type: 'reorder', from: 0, to: 4 })).toBe(state);
  });
});
