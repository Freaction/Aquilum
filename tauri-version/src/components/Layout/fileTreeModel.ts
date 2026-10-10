import type { WorkspaceItem } from '../../modules/documents/fileGateway';
import type { FileMenuActions } from './fileActionItems';
import { comparablePath, relativePath } from '../../modules/paths';
import type { ColorToken } from '../../plugins/vaultData';

export interface VisibleFileRow {
  item: WorkspaceItem;
  comparableId: string;
  depth: number;
  expanded: boolean;
  loading: boolean;
  guideDepths: string;
  isHidden?: boolean;
  color?: ColorToken;
  colorBackground?: boolean;
  iconName?: string;
  count?: number;
}

export interface ExplorerTreeOptions {
  workspacePath: string;
  hidden: ReadonlySet<string>;
  pinned: ReadonlySet<string>;
  showHidden: boolean;
}

export interface FileTreeActions extends FileMenuActions {
  selectFile: (path: string, isMulti: boolean, isRange: boolean) => void;
  openInNewTab: (path: string) => void;
  focusRow: (path: string) => void;
  toggleFolder: (id: string) => void;
  prefetchFolder: (id: string) => void;
  commitRename: (path: string, nextName: string) => void;
  cancelRename: () => void;
}

export function parseGuideDepths(guideDepths: string): number[] {
  return guideDepths === '' ? [] : guideDepths.split(',').map(Number);
}

export function buildVisibleFileRows(
  roots: readonly WorkspaceItem[],
  directories: ReadonlyMap<string, WorkspaceItem[]>,
  expandedFolders: ReadonlySet<string>,
  loadingDirectories: ReadonlySet<string>,
  explorer?: ExplorerTreeOptions,
): VisibleFileRow[] {
  const rows: VisibleFileRow[] = [];

  const visit = (items: readonly WorkspaceItem[], depth: number, continuingGuides: number[]) => {
    const guideDepths = continuingGuides.join(',');

    const visible = items.map(item => {
      const path = explorer ? relativePath(explorer.workspacePath, item.id) : '';
      let ancestor = path;
      let isHidden = false;
      while (ancestor) {
        if (explorer?.hidden.has(ancestor)) { isHidden = true; break; }
        ancestor = ancestor.includes('/') ? ancestor.slice(0, ancestor.lastIndexOf('/')) : '';
      }
      return { item, path, isHidden };
    }).filter(row => !row.isHidden || explorer?.showHidden);
    if (explorer) visible.sort((a, b) => (
      Number(a.item.type !== 'folder') - Number(b.item.type !== 'folder')
      || Number(explorer.pinned.has(b.path)) - Number(explorer.pinned.has(a.path))
    ));
    visible.forEach(({ item, isHidden }, index) => {
      const isFolder = item.type === 'folder';
      const expanded = isFolder && expandedFolders.has(item.id);
      const isLast = index === visible.length - 1;

      rows.push({
        item,
        comparableId: comparablePath(item.id),
        depth,
        expanded,
        loading: isFolder && loadingDirectories.has(item.id),
        guideDepths,
        isHidden,
      });

      if (expanded) {
        const children = directories.get(item.id);
        if (children && children.length > 0) {
          let guidesForChildren = [...continuingGuides];
          if (isLast) {
            guidesForChildren = guidesForChildren.filter(g => g !== depth - 1);
          }
          guidesForChildren.push(depth);
          visit(children, depth + 1, guidesForChildren);
        }
      }
    });
  };

  visit(roots, 0, []);
  return rows;
}

export function unloadedExpandedFolders(
  rows: readonly VisibleFileRow[],
  directories: ReadonlyMap<string, WorkspaceItem[]>,
): string[] {
  return rows
    .filter((row) => row.expanded && !directories.has(row.item.id))
    .map((row) => row.item.id);
}
