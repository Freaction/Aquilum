import { t } from '../../i18n';
import { SidebarDocumentItem } from '../Backlinks/SidebarDocumentItem';
import { InlineRenameField } from '../Common/InlineRenameField';
import { Menu, type MenuItem } from '../Common/Menu';
import { useContextMenu } from '../Common/useContextMenu';

interface HistoryRowProps {
  label: string;
  counter?: string;
  title?: string;
  selected: boolean;
  renaming?: boolean;
  menuItems?: MenuItem[];
  onOpen: () => void;
  onRename?: (name: string) => void;
  onCancelRename?: () => void;
}

export function HistoryRow({
  label,
  counter,
  title,
  selected,
  renaming = false,
  menuItems = [],
  onOpen,
  onRename,
  onCancelRename,
}: HistoryRowProps) {
  const menu = useContextMenu();
  if (renaming && onRename && onCancelRename) {
    return (
      <div className="q-sidebar-document-item q-history-list__item" aria-current={selected}>
        <InlineRenameField
          name={label}
          ariaLabel={t('history.renameAria')}
          className="q-history-list__rename"
          onCommit={onRename}
          onCancel={onCancelRename}
        />
      </div>
    );
  }
  return (
    <>
      <SidebarDocumentItem
        label={label}
        counter={counter}
        title={title}
        className="q-history-list__item"
        aria-current={selected}
        onClick={onOpen}
        onContextMenu={menuItems.length > 0 ? menu.onContextMenu : undefined}
      />
      {menu.open && menuItems.length > 0 && (
        <Menu
          open
          position={menu.position}
          items={menuItems}
          onClose={menu.close}
          ariaLabel={t('history.versionActions')}
        />
      )}
    </>
  );
}
