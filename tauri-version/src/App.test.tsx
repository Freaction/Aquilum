// @vitest-environment happy-dom
import { act } from 'preact/test-utils';
import { useEffect, type ReactNode } from 'react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import App from './App';
import { EditorState } from '@codemirror/state';
import { EditorView } from '@codemirror/view';
import { editorMarkdownSupport } from './components/Editor/extensions/markdownConfig';
import { bodySetup } from './components/Editor/extensions/bodySetup';
import { WidgetSession } from './components/Editor/extensions/tables/widgetSession';
import { findTablesInDoc } from './components/Editor/extensions/tables/constructs';
import { renderTableWidgetDom } from './components/Editor/extensions/tables/widgetDom';
import { getReadingMode, setReadingMode } from './plugins/editor/readingMode';
import { legacyConfig } from './plugins/testFixtures';
import { DEFAULT_PLUGIN_SETTINGS } from './modules/settings';
import { actAndSettle, mountDom, type MountedDom } from './testing/mountDom';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn(async () => undefined) }));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(async () => () => {}) }));

const settingsState = vi.hoisted(() => ({ config: null as any, updateConfig: vi.fn() }));
const editorLifecycle: string[] = [];
const graphLifecycle: string[] = [];
const workspaceState = vi.hoisted(() => ({
  files: [] as Array<{ id: string; name: string; type: 'file' }>,
  workspaceReady: false,
}));
const uiStateMocks = vi.hoisted(() => ({
  markDocumentMissing: vi.fn(),
  open: vi.fn(),
  queueActiveTab: vi.fn(),
  queueTabsSnapshot: vi.fn(),
}));

vi.mock('./hooks/useWorkspace', () => ({
  useWorkspace: () => ({
    directories: new Map(),
    files: workspaceState.files,
    loadingDirectories: new Set(),
    workspacePath: 'C:\\notes',
    workspaceName: 'notes',
    workspaceReady: workspaceState.workspaceReady,
    workspaceRestoring: false,
    homePage: '',
    changeHomePage: vi.fn(),
    openWorkspace: vi.fn(),
    loadDirectory: vi.fn(),
    patchFileInTree: vi.fn(),
  }),
}));

vi.mock('./modules/documents/fileGateway', async (importOriginal) => {
  const actual = await importOriginal<typeof import('./modules/documents/fileGateway')>();
  return {
    ...actual,
    existingFiles: async (paths: string[]) => {
      const available = new Set(workspaceState.files.map((file) => file.id));
      return paths.filter((path) => available.has(path));
    },
  };
});

vi.mock('./modules/ui-state', async (importOriginal) => {
  const actual = await importOriginal<typeof import('./modules/ui-state')>();
  return {
    ...actual,
    WorkspaceSession: {
      open: uiStateMocks.open,
      restorable: actual.WorkspaceSession.restorable,
    },
  };
});

vi.mock('./modules/windowReveal', () => ({
  revealAppWindow: vi.fn(() => Promise.resolve()),
}));

vi.mock('./components/Layout/Titlebar', () => ({
  Titlebar: ({
    onToggleRightSidebar,
  }: {
    onToggleRightSidebar: () => void;
  }) => (
    <>
      <button id="toggle-right-sidebar" onClick={onToggleRightSidebar} />
    </>
  ),
}));

vi.mock('./components/Layout/WindowControls', () => ({
  WindowControls: () => null,
}));

vi.mock('./components/Layout/SidebarRail', () => ({
  SidebarRail: ({
    onToggleSidebar,
    onOpenGraph,
  }: {
    onToggleSidebar: () => void;
    onOpenGraph: () => void;
  }) => (
    <>
      <button id="hide-left-sidebar" onClick={onToggleSidebar} />
      <button id="open-graph" onClick={onOpenGraph} />
    </>
  ),
}));

vi.mock('./components/Graph/GraphView', () => ({
  GraphView: ({ inactive }: { inactive: boolean }) => {
    useEffect(() => {
      graphLifecycle.push('mount');
      return () => {
        graphLifecycle.push('unmount');
      };
    }, []);
    return <div id="graph" data-inactive={inactive} />;
  },
}));

vi.mock('./components/Layout/Sidebar', () => ({
  Sidebar: ({
    onFileSelect,
    isOpen,
  }: {
    onFileSelect: (path: string) => void;
    isOpen: boolean;
  }) => isOpen ? (
    <div id="left-sidebar">
      <button id="open-a" onClick={() => onFileSelect('C:\\notes\\a.md')} />
      <button id="open-b" onClick={() => onFileSelect('C:\\notes\\b.md')} />
    </div>
  ) : null,
}));

vi.mock('./components/Editor/NewTab', () => ({
  NewTab: ({ onOpen }: { onOpen: () => void }) => (
    <button id="open-file-from-new-tab" onClick={onOpen} />
  ),
}));

vi.mock('./components/Search/SearchDialog', () => ({
  SearchDialog: ({ open }: { open: boolean }) => <div id="search-dialog" data-open={open} />,
}));

vi.mock('./components/Settings/SettingsDialog', () => ({
  SettingsDialog: ({ open }: { open: boolean }) => <div id="settings-dialog" data-open={open} />,
}));

vi.mock('./modules/settings', async importOriginal => ({
  ...await importOriginal<typeof import('./modules/settings')>(),
  DEFAULT_LIVE_TABS: 3,
  useSettingsStore: () => ({
    config: settingsState.config,
    isLoading: false,
    error: null,
    loadConfig: () => Promise.resolve(),
    updateConfig: settingsState.updateConfig,
  }),
}));

vi.mock('./components/Backlinks/BacklinksPanel', () => ({
  BacklinksPanel: ({ isOpen }: { isOpen: boolean }) => isOpen ? <div id="right-sidebar" /> : null,
}));

vi.mock('./components/Common/ErrorBoundary', () => ({
  ErrorBoundary: ({ children }: { children: ReactNode }) => children,
}));

vi.mock('./components/Editor', () => ({
  Editor: ({ filePath }: { filePath: string }) => {
    useEffect(() => {
      editorLifecycle.push(`mount:${filePath}`);
      return () => {
        editorLifecycle.push(`unmount:${filePath}`);
      };
    }, []);
    return <div>{filePath}</div>;
  },
}));

function sessionStub(overrides: Record<string, unknown> = {}) {
  return {
    loaded: { activeTabId: null },
    restoredTabs: () => [],
    markDocumentMissing: uiStateMocks.markDocumentMissing,
    queueActiveTab: uiStateMocks.queueActiveTab,
    queueTabsSnapshot: uiStateMocks.queueTabsSnapshot,
    queueView: vi.fn(),
    loadedView: vi.fn(() => null),
    isViewLoaded: vi.fn(() => true),
    graphCamera: vi.fn(() => null),
    queueGraphCamera: vi.fn(),
    ensureViewLoaded: vi.fn(async () => null),
    resolveDocument: vi.fn(async (path: string) => `doc:${path}`),
    renameDocument: vi.fn(),
    dispose: vi.fn(),
    ...overrides,
  };
}

describe('App editor lifecycle', () => {
  let renderer: MountedDom | null = null;

  const find = (id: string) => renderer!.container.querySelectorAll<HTMLElement>(`#${id}`);
  const attribute = (id: string, name: string) => find(id)[0]?.getAttribute(name);
  const click = (id: string) => act(() => find(id)[0].click());

  async function renderWithSession(node = <App />): Promise<MountedDom> {
    workspaceState.workspaceReady = true;
    uiStateMocks.open.mockResolvedValue(sessionStub());
    await actAndSettle(() => {
      renderer = mountDom(node);
    });
    return renderer!;
  }

  afterEach(() => {
    if (renderer) {
      act(() => renderer?.unmount());
      renderer = null;
    }
    editorLifecycle.length = 0;
    graphLifecycle.length = 0;
    workspaceState.files = [];
    workspaceState.workspaceReady = false;
    settingsState.config = null;
    setReadingMode(false);
    vi.clearAllMocks();
  });

  it('keeps the editor area empty until the workspace session is restored', () => {
    act(() => {
      renderer = mountDom(<App />);
    });

    expect(find('open-file-from-new-tab')).toHaveLength(0);
    expect(editorLifecycle).toHaveLength(0);
  });

  it('creates a fresh editor instance when a sidebar selection replaces the active file', async () => {
    await renderWithSession();

    click('open-a');
    click('open-b');

    const unmountedA = editorLifecycle.lastIndexOf('unmount:C:\\notes\\a.md');
    const mountedB = editorLifecycle.lastIndexOf('mount:C:\\notes\\b.md');
    expect(unmountedA).toBeGreaterThanOrEqual(0);
    expect(mountedB).toBeGreaterThan(unmountedA);
  });

  it('keeps the graph mounted and merely hidden while another tab is active', async () => {
    await renderWithSession();

    click('open-a');
    await actAndSettle(() => find('open-graph')[0].click());
    expect(graphLifecycle).toEqual(['mount']);
    expect(attribute('graph', 'data-inactive')).toBe('false');

    click('open-a');
    expect(graphLifecycle).toEqual(['mount']);
    expect(attribute('graph', 'data-inactive')).toBe('true');

    click('open-graph');
    expect(graphLifecycle).toEqual(['mount']);
    expect(attribute('graph', 'data-inactive')).toBe('false');
  });

  it('opens global search with the physical Ctrl+O shortcut', () => {
    act(() => {
      renderer = mountDom(<App />);
    });
    const shortcut = new KeyboardEvent('keydown', {
      code: 'KeyO',
      key: 'щ',
      ctrlKey: true,
      cancelable: true,
    });

    act(() => {
      window.dispatchEvent(shortcut);
    });

    expect(shortcut.defaultPrevented).toBe(true);
    expect(attribute('search-dialog', 'data-open')).toBe('true');
  });


  it('toggles full width with physical Meta+Alt+KeyW from an editor', async () => {
    settingsState.config = { ...legacyConfig(), plugins: structuredClone(DEFAULT_PLUGIN_SETTINGS) };
    settingsState.updateConfig.mockImplementation(async next => { settingsState.config = next; renderer!.update(<App />); });
    await renderWithSession();
    const view = new EditorView({ parent: document.body, state: EditorState.create({ doc: 'note', extensions: [bodySetup, editorMarkdownSupport] }) });
    try {
      view.focus();
      for (const fullWidth of [true, false]) {
        const event = new KeyboardEvent('keydown', { code: 'KeyW', key: 'ц', metaKey: true, altKey: true, bubbles: true, cancelable: true });
        await actAndSettle(() => { view.contentDOM.dispatchEvent(event); });
        expect(event.defaultPrevented).toBe(true);
        expect(settingsState.config.editor.fullWidth).toBe(fullWidth);
      }
    } finally { view.destroy(); }
  });

  it('toggles reading mode from CodeMirror and a focused nested table cell', async () => {
    settingsState.config = { ...legacyConfig(), plugins: structuredClone(DEFAULT_PLUGIN_SETTINGS) };
    settingsState.config.plugins.readingMode.enabled = true;
    await renderWithSession();
    const doc = '| name |\n| --- |\n| value |';
    const view = new EditorView({ parent: document.body, state: EditorState.create({ doc, extensions: [bodySetup, editorMarkdownSupport] }) });
    const table = findTablesInDoc(view.state.doc)[0];
    const root = renderTableWidgetDom(table.model);
    view.dom.append(root);
    const session = new WidgetSession(view, { from: 0, contentTo: table.contentTo, blockTo: table.to }, table.model, root);
    root.addEventListener('keydown', event => session.onKeyDown(event));
    const toggle = async (target: HTMLElement, expected: boolean) => {
      target.focus();
      const event = new KeyboardEvent('keydown', { code: 'KeyE', key: 'E', metaKey: true, shiftKey: true, bubbles: true, cancelable: true });
      await actAndSettle(() => { target.dispatchEvent(event); });
      expect(event.defaultPrevented).toBe(true);
      expect(getReadingMode()).toBe(expected);
    };
    try {
      await toggle(view.contentDOM, true);
      session.openCellEditor({ row: 1, col: 0 });
      const nested = EditorView.findFromDOM(root.querySelector('.cm-editor')!)!;
      await toggle(nested.contentDOM, false);
      await toggle(nested.contentDOM, true);
    } finally { session.dispose(); view.destroy(); root.remove(); }
  });


  it('uses configured reading and width shortcuts and releases their defaults', async () => {
    settingsState.config = { ...legacyConfig(), plugins: structuredClone(DEFAULT_PLUGIN_SETTINGS) };
    settingsState.config.editor.fullWidthShortcut = { code: 'KeyJ', key: 'J', primary: true, alt: true, shift: false };
    settingsState.config.plugins.readingMode.enabled = true;
    settingsState.config.plugins.readingMode.shortcut = { code: 'KeyK', key: 'K', primary: true, alt: true, shift: false };
    settingsState.updateConfig.mockImplementation(async next => { settingsState.config = next; renderer!.update(<App />); });
    await renderWithSession();
    const press = (code: string, modifiers = {}) => actAndSettle(() => { window.dispatchEvent(new KeyboardEvent('keydown', { code, key: code.slice(3), metaKey: true, altKey: true, cancelable: true, ...modifiers })); });
    await press('KeyW');
    await press('KeyE', { altKey: false, shiftKey: true });
    expect(settingsState.config.editor.fullWidth).toBe(false);
    expect(getReadingMode()).toBe(false);
    await press('KeyJ');
    expect(settingsState.config.editor.fullWidth).toBe(true);
    await press('KeyK');
    expect(getReadingMode()).toBe(true);
  });

  it('opens search from a new tab', async () => {
    await renderWithSession();

    click('open-file-from-new-tab');

    expect(attribute('search-dialog', 'data-open')).toBe('true');
  });

  it('hides and restores both sidebars', () => {
    act(() => {
      renderer = mountDom(<App />);
    });

    expect(find('left-sidebar')).toHaveLength(1);
    expect(find('right-sidebar')).toHaveLength(1);

    click('hide-left-sidebar');
    expect(find('left-sidebar')).toHaveLength(0);

    click('hide-left-sidebar');
    expect(find('left-sidebar')).toHaveLength(1);

    click('toggle-right-sidebar');
    expect(find('right-sidebar')).toHaveLength(0);

    click('toggle-right-sidebar');
    expect(find('right-sidebar')).toHaveLength(1);
  });

  it('restores an existing tab and rejects a missing active tab', async () => {
    workspaceState.workspaceReady = true;
    workspaceState.files = [{ id: 'C:\\notes\\kept.md', name: 'kept.md', type: 'file' }];
    uiStateMocks.markDocumentMissing.mockResolvedValue(undefined);
    uiStateMocks.open.mockResolvedValue({
      loaded: { activeTabId: 'missing-tab' },
      restoredTabs: () => [
        { tabId: 'kept-tab', documentId: 'kept-doc', kind: 'document', path: 'C:\\notes\\kept.md' },
        { tabId: 'missing-tab', documentId: 'missing-doc', kind: 'document', path: 'C:\\notes\\missing.md' },
      ],
      markDocumentMissing: uiStateMocks.markDocumentMissing,
      queueActiveTab: uiStateMocks.queueActiveTab,
      queueTabsSnapshot: uiStateMocks.queueTabsSnapshot,
      queueView: vi.fn(),
      loadedView: vi.fn(() => null),
      isViewLoaded: vi.fn(() => true),
      graphCamera: vi.fn(() => null),
      queueGraphCamera: vi.fn(),
      ensureViewLoaded: vi.fn(async () => null),
      resolveDocument: vi.fn(),
      renameDocument: vi.fn(),
      dispose: vi.fn(),
    });

    await actAndSettle(() => {
      renderer = mountDom(<App />);
    });

    expect(editorLifecycle[editorLifecycle.length - 1]).toBe('mount:C:\\notes\\kept.md');
    expect(uiStateMocks.markDocumentMissing).toHaveBeenCalledWith('missing-doc');
    expect(uiStateMocks.queueTabsSnapshot).toHaveBeenLastCalledWith(
      [expect.objectContaining({ tabId: 'kept-tab' })],
      'kept-tab',
    );
  });
});
