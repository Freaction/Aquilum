import { memo, type CSSProperties } from 'react';
import { ChevronDown, ChevronRight, LoaderCircle } from 'lucide';
import { Icon } from '../Common/Icon';
import { isMarkdownPath, type WorkspaceItem } from '../../modules/documents/fileGateway';
import { fileActionItems } from './fileActionItems';
import { parseGuideDepths, type FileTreeActions } from './fileTreeModel';
import { FileTreeRename } from './FileTreeRename';
import { Menu } from '../Common/Menu';
import { t } from '../../i18n';
import { titleWhenClipped } from '../Common/titleWhenClipped';
import { useContextMenu } from '../Common/useContextMenu';
import { iconByName } from '../../plugins/explorer/icons';
import type { ColorToken } from '../../plugins/vaultData';

interface FileTreeRowProps {
  item: WorkspaceItem;
  depth: number;
  expanded: boolean;
  loading: boolean;
  active: boolean;
  selected: boolean;
  guideDepths: string;
  renaming: boolean;
  actions: FileTreeActions;
  isHidden?: boolean;
  color?: ColorToken;
  colorBackground?: boolean;
  iconName?: string;
  count?: number;
}

export const FileTreeRow = memo(function FileTreeRow({
  item,
  depth,
  expanded,
  loading,
  active,
  selected,
  guideDepths,
  renaming,
  actions,
  isHidden,
  color,
  colorBackground,
  iconName,
  count,
}: FileTreeRowProps) {
  const isFolder = item.type === 'folder';
  const allowFileActions = isFolder || isMarkdownPath(item.id);
  const canRowActions = allowFileActions || !!(actions.createNoteIn || actions.createFolderIn || actions.copyVaultPath || actions.copySystemPath || actions.revealInFileManager
    || actions.chooseColor || actions.chooseIcon || actions.togglePin || actions.hidePath);
  const customIcon = iconName ? iconByName(iconName) : undefined;
  const menu = useContextMenu();

  const itemClass = [
    'q-file-item',
    active ? 'active' : '',
    selected ? 'selected' : '',
    renaming ? 'renaming' : '',
    isHidden ? 'is-hidden' : '',
    color ? 'q-file-colored' : '',
    color && colorBackground ? 'q-file-colored-background' : '',
  ].filter(Boolean).join(' ');

  return (
    <div
      className="q-file-tree-node"
      data-file-id={item.id}
      data-file-type={item.type}
      data-file-name={item.name}
      data-renaming={renaming ? '' : undefined}
      style={{ '--q-file-depth': depth, '--q-file-color': color ? `var(--q-${color}-500)` : undefined } as CSSProperties}
    >
      {parseGuideDepths(guideDepths).map((guideDepth) => (
        <div
          key={guideDepth}
          className="q-file-tree-guide"
          style={{ '--q-guide-depth': guideDepth } as CSSProperties}
        />
      ))}
      {renaming ? (
        <FileTreeRename
          name={item.name}
          className={itemClass}
          active={active}
          onCommit={(nextName) => actions.commitRename(item.id, nextName)}
          onCancel={actions.cancelRename}
        />
      ) : (
        <button
          type="button"
          className={itemClass}
          data-file-active={active || undefined}
          aria-expanded={isFolder ? expanded : undefined}
          onMouseEnter={() => {
            if (isFolder) actions.prefetchFolder(item.id);
          }}
          onClick={(event) => {
            const primary = event.ctrlKey || event.metaKey;
            if (primary && event.shiftKey) {
              actions.openInNewTab(item.id);
            } else if (isFolder && !primary && !event.shiftKey) {
              actions.toggleFolder(item.id);
            } else {
              actions.selectFile(item.id, primary, event.shiftKey);
            }
          }}
          onAuxClick={(event) => {
            if (event.button === 1 && !isFolder) actions.openInNewTab(item.id);
          }}
          onContextMenu={(event) => {
            if (!canRowActions) return;
            actions.focusRow(item.id);
            menu.onContextMenu(event);
          }}
        >
          <span className="q-file-icon" aria-hidden="true">
            {isFolder && (loading
              ? <Icon icon={LoaderCircle} className="q-file-icon__spinner" />
              : expanded ? <Icon icon={ChevronDown} /> : <Icon icon={ChevronRight} />)}
          </span>
          {customIcon && <span className="q-file-custom-icon" aria-hidden="true"><Icon icon={customIcon} /></span>}
          <span
            className="q-file-name"
            onMouseEnter={(event) => titleWhenClipped(event.currentTarget, item.name)}
          >
            {item.name}
          </span>
          {isFolder && count !== undefined && <span className="q-file-count">{count}</span>}
        </button>
      )}
      {canRowActions && menu.open && (
        <Menu
          open
          position={menu.position}
          items={fileActionItems(actions, item.id, allowFileActions)}
          onClose={menu.close}
          ariaLabel={isFolder ? t('fileTree.folderActions') : t('fileTree.fileActions')}
        />
      )}
    </div>
  );
});
