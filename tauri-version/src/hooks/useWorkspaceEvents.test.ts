import { describe, expect, it } from 'vitest';
import type { WorkspaceItem } from '../modules/documents/fileGateway';
import { affectedDirectories } from './useWorkspaceEvents';

describe('workspace change routing', () => {
  it('refreshes only loaded parent directories', () => {
    const root = 'C:\\vault';
    const folder = 'C:\\vault\\folder';
    const collapsed = 'C:\\vault\\collapsed';
    const directories = new Map<string, WorkspaceItem[]>([
      [root, [
        { id: folder, name: 'folder', type: 'folder' },
        { id: collapsed, name: 'collapsed', type: 'folder' },
      ]],
      [folder, [{ id: `${folder}\\note.md`, name: 'note', type: 'file' }]],
    ]);

    expect(affectedDirectories(root, directories, [folder])).toEqual([root]);
    expect(affectedDirectories(root, directories, [`${folder}\\note.md`])).toEqual([folder]);
    expect(affectedDirectories(root, directories, [root])).toEqual([root]);
    expect(affectedDirectories(root, directories, [`${collapsed}\\note.md`])).toEqual([]);
  });

  it('shows a folder the tree has never seen', () => {
    const root = 'C:\\vault';
    const directories = new Map<string, WorkspaceItem[]>([[root, []]]);

    expect(affectedDirectories(root, directories, ['C:\\vault\\Templates\\Книга.md']))
      .toEqual([root]);
    expect(affectedDirectories(root, directories, ['C:\\vault\\Templates\\Глубже\\Книга.md']))
      .toEqual([root]);
  });
});
