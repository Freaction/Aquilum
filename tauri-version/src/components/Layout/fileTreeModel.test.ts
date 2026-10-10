import { describe, expect, it } from 'vitest';
import type { WorkspaceItem } from '../../modules/documents/fileGateway';
import { buildVisibleFileRows, parseGuideDepths, unloadedExpandedFolders } from './fileTreeModel';

const folder: WorkspaceItem = { id: 'C:\\vault\\folder', name: 'folder', type: 'folder' };
const rootFile: WorkspaceItem = { id: 'C:\\vault\\root.md', name: 'root', type: 'file' };
const childFile: WorkspaceItem = { id: 'C:\\vault\\folder\\child.md', name: 'child', type: 'file' };
const nested: WorkspaceItem = { id: 'C:\\vault\\folder\\nested', name: 'nested', type: 'folder' };
const nestedFile: WorkspaceItem = { id: 'C:\\vault\\folder\\nested\\deep.md', name: 'deep', type: 'file' };

describe('file tree model', () => {
  it('keeps collapsed children out of the visible rows', () => {
    const rows = buildVisibleFileRows(
      [folder, rootFile],
      new Map([[folder.id, [childFile]]]),
      new Set(),
      new Set(),
    );

    expect(rows.map((row) => row.item.id)).toEqual([folder.id, rootFile.id]);
  });

  it('adds expanded children at the next depth', () => {
    const rows = buildVisibleFileRows(
      [folder],
      new Map([[folder.id, [childFile]]]),
      new Set([folder.id]),
      new Set(),
    );

    expect(rows.map((row) => [row.item.id, row.depth])).toEqual([
      [folder.id, 0],
      [childFile.id, 1],
    ]);
  });

  it('describes guide lines as a primitive so memoised rows survive a rebuild', () => {
    const args = [
      [folder, rootFile],
      new Map([[folder.id, [nested, childFile]], [nested.id, [nestedFile]]]),
      new Set([folder.id, nested.id]),
      new Set<string>(),
    ] as const;

    const rows = buildVisibleFileRows(...args);
    const rebuilt = buildVisibleFileRows(...args);

    expect(rows.map((row) => row.guideDepths)).toEqual(['', '0', '0,1', '0', '']);
    expect(rows.map((row) => row.guideDepths)).toEqual(rebuilt.map((row) => row.guideDepths));
    expect(parseGuideDepths(rows[2].guideDepths)).toEqual([0, 1]);
    expect(parseGuideDepths('')).toEqual([]);
  });

  it('names an expanded folder whose contents are not loaded yet', () => {
    const directories = new Map([[nested.id, [nestedFile]]]);
    const rows = buildVisibleFileRows([folder], directories, new Set([folder.id]), new Set());

    expect(unloadedExpandedFolders(rows, directories)).toEqual([folder.id]);
    expect(unloadedExpandedFolders(rows, new Map([[folder.id, []]]))).toEqual([]);
  });
});

describe('explorer filters in visible rows', () => {
  const options = { workspacePath: 'C:/vault', hidden: new Set(['folder']), pinned: new Set<string>(), showHidden: false };
  const directories = new Map([[folder.id, [childFile, nested]], [nested.id, [nestedFile]]]);
  const expanded = new Set([folder.id, nested.id]);

  it('hides matching folders with all descendants and matching files', () => {
    const rows = buildVisibleFileRows([folder, rootFile], directories, expanded, new Set(), options);
    expect(rows.map(row => row.item.id)).toEqual([rootFile.id]);
    const files = buildVisibleFileRows([folder, rootFile], directories, expanded, new Set(), { ...options, hidden: new Set(['folder/child.md']) });
    expect(files.map(row => row.item.id)).toEqual([folder.id, nested.id, nestedFile.id, rootFile.id]);
  });

  it('shows hidden rows and marks descendants without changing counts', () => {
    const rows = buildVisibleFileRows([folder, rootFile], directories, expanded, new Set(), { ...options, showHidden: true });
    expect(rows.map(row => [row.item.id, row.isHidden])).toEqual([
      [folder.id, true], [nested.id, true], [nestedFile.id, true], [childFile.id, true], [rootFile.id, false],
    ]);
  });

  it('puts pinned folders before other folders and pinned files before other files in each folder', () => {
    const otherFolder: WorkspaceItem = { id: 'C:/vault/other', name: 'other', type: 'folder' };
    const otherFile: WorkspaceItem = { id: 'C:/vault/other.md', name: 'other', type: 'file' };
    const rows = buildVisibleFileRows([folder, otherFolder, rootFile, otherFile], directories, expanded, new Set(), {
      ...options, hidden: new Set(), pinned: new Set(['other', 'other.md', 'folder/nested']),
    });
    expect(rows.map(row => row.item.id)).toEqual([otherFolder.id, folder.id, nested.id, nestedFile.id, childFile.id, otherFile.id, rootFile.id]);
    expect(rows.find(row => row.item.id === otherFile.id)?.depth).toBe(0);
  });

  it('keeps the input arrays unchanged and respects exact path boundaries', () => {
    const roots = [folder, rootFile];
    const rows = buildVisibleFileRows(roots, directories, expanded, new Set(), { ...options, hidden: new Set(['fold']) });
    expect(rows.map(row => row.item.id)).toContain(folder.id);
    expect(roots).toEqual([folder, rootFile]);
  });
});
