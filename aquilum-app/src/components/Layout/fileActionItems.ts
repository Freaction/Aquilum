import { t } from '../../i18n';
import { isMacOs } from '../../modules/platform';
import type { MenuItem } from '../Common/Menu';

export interface FileMenuActions {
  startRename: (path: string) => void;
  duplicateFile: (path: string) => void;
  requestDelete: (path: string) => void;
  chooseColor?: (path: string) => void;
  chooseIcon?: (path: string) => void;
  togglePin?: (path: string) => void;
  hidePath?: (path: string) => void;
  isPinned?: (path: string) => boolean;
  isHidden?: (path: string) => boolean;
  createNoteIn?: (path: string) => void;
  createFolderIn?: (path: string) => void;
  copyVaultPath?: (path: string) => void;
  copySystemPath?: (path: string) => void;
  revealInFileManager?: (path: string) => void;
}

export function renameItem(onSelect: () => void): MenuItem {
  return { id: 'rename', label: t('common.rename'), onSelect };
}

export function fileActionItems(actions: FileMenuActions, path: string, allowFileActions = true): MenuItem[] {
  const items: MenuItem[] = [];
  if (actions.createNoteIn) items.push({ id: 'new-note', label: t('fileTree.newNote'), onSelect: () => actions.createNoteIn!(path) });
  if (actions.createFolderIn) items.push({ id: 'new-folder', label: t('fileTree.newFolder'), onSelect: () => actions.createFolderIn!(path) });
  if (allowFileActions) items.push(
    renameItem(() => actions.startRename(path)),
    { id: 'duplicate', label: t('common.duplicate'), onSelect: () => actions.duplicateFile(path) },
    { id: 'delete', label: t('common.delete'), onSelect: () => actions.requestDelete(path) },
  );
  const copyItems = [
    actions.copyVaultPath && { id: 'copy-vault-path', label: t('fileTree.copyPathFromVault'), onSelect: () => actions.copyVaultPath!(path) },
    actions.copySystemPath && { id: 'copy-system-path', label: t('fileTree.copyPathAbsolute'), onSelect: () => actions.copySystemPath!(path) },
  ].filter((item): item is MenuItem => Boolean(item));
  if (copyItems.length) items.push({ id: 'copy-path', label: t('fileTree.copyPath'), children: copyItems, onSelect: () => {} });
  if (actions.revealInFileManager) items.push({ id: 'reveal', label: t(isMacOs() ? 'fileTree.revealInFinder' : 'fileTree.revealInFileManager'), onSelect: () => actions.revealInFileManager!(path) });
  // пункты появляются только при включённом плагине.
  if (actions.chooseColor) items.push({ id: 'color', label: t('plugins.fileColors.choose'), onSelect: () => actions.chooseColor!(path) });
  if (actions.chooseIcon) items.push({ id: 'icon', label: t('plugins.fileIcons.choose'), onSelect: () => actions.chooseIcon!(path) });
  if (actions.togglePin) {
    const pinned = actions.isPinned?.(path) ?? false;
    items.push({ id: pinned ? 'unpin' : 'pin', label: t(pinned ? 'plugins.explorerFilters.unpin' : 'plugins.explorerFilters.pin'), onSelect: () => actions.togglePin!(path) });
  }
  if (actions.hidePath) {
    const hidden = actions.isHidden?.(path) ?? false;
    items.push({ id: hidden ? 'unhide' : 'hide', label: t(hidden ? 'plugins.explorerFilters.unhide' : 'plugins.explorerFilters.hide'), onSelect: () => actions.hidePath!(path) });
  }
  return items;
}
