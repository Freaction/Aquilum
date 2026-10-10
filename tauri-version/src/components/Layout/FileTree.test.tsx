// @vitest-environment happy-dom
import { act } from 'preact/test-utils';
import { mountDom, type MountedDom } from '../../testing/mountDom';
import { describe, expect, it, vi } from 'vitest';
import type { WorkspaceItem } from '../../modules/documents/fileGateway';
import type { FileTreeActions } from './fileTreeModel';

vi.mock('./fileTreeModel', async (importOriginal) => {
  const actual = await importOriginal<typeof import('./fileTreeModel')>();
  return { ...actual, parseGuideDepths: vi.fn(actual.parseGuideDepths) };
});

const { buildVisibleFileRows, parseGuideDepths } = await import('./fileTreeModel');
const { FileTree } = await import('./FileTree');

const notes: WorkspaceItem[] = Array.from({ length: 40 }, (_, index) => ({
  id: `C:\\vault\\note-${index}.md`,
  name: `note-${index}`,
  type: 'file',
}));

const noop = () => {};
const rows = buildVisibleFileRows(notes, new Map(), new Set(), new Set());
const actions: FileTreeActions = {
  selectFile: noop,
  openInNewTab: noop,
  focusRow: noop,
  toggleFolder: noop,
  prefetchFolder: noop,
  startRename: noop,
  commitRename: noop,
  cancelRename: noop,
  requestDelete: noop,
  duplicateFile: noop,
};

function renderTree(activeFile: string | null, selectedFiles: ReadonlySet<string> = new Set<string>()) {
  return (
    <FileTree
      rows={rows}
      activeFile={activeFile}
      selectedFiles={selectedFiles}
      renamingPath={null}
      actions={actions}
    />
  );
}

const selectedRows = (tree: MountedDom) => tree.container.querySelectorAll('.selected');

const renderedRows = () => vi.mocked(parseGuideDepths).mock.calls.length;

describe('file tree rendering', () => {
  it('re-renders only the two affected rows when the active file changes', () => {
    let tree!: MountedDom;
    act(() => {
      tree = mountDom(renderTree(notes[0].id));
    });
    expect(renderedRows()).toBe(notes.length);

    const afterMount = renderedRows();
    act(() => {
      tree.update(renderTree(notes[7].id));
    });

    expect(renderedRows() - afterMount).toBe(2);
    act(() => {
      tree.unmount();
    });
  });

  it('highlights the very first picked row, not only from the second one', () => {
    let tree!: MountedDom;
    act(() => {
      tree = mountDom(renderTree(notes[0].id, new Set([notes[5].id])));
    });
    expect(selectedRows(tree)).toHaveLength(1);

    act(() => {
      tree.update(renderTree(notes[0].id, new Set([notes[5].id, notes[6].id])));
    });
    expect(selectedRows(tree)).toHaveLength(2);
    act(() => {
      tree.unmount();
    });
  });

  it('marks the active row for the scroll-into-view lookup', () => {
    let tree!: MountedDom;
    act(() => {
      tree = mountDom(renderTree('c:/vault/note-3.md'));
    });

    const active = tree.container.querySelectorAll('[data-file-active="true"]');
    expect(active).toHaveLength(1);
    expect(active[0].classList).toContain('active');
    expect(active[0].querySelector('.q-file-name')?.textContent).toBe('note-3');
    act(() => {
      tree.unmount();
    });
  });
});
