import { memo, MouseEvent, KeyboardEvent } from 'react';
import { Network, X } from 'lucide';
import { Icon } from '../Common/Icon';
import { IconButton } from '../Common/IconButton';
import { InlineRenameField } from '../Common/InlineRenameField';
import { Menu } from '../Common/Menu';
import { t } from '../../i18n';
import { titleWhenClipped } from '../Common/titleWhenClipped';
import { useContextMenu } from '../Common/useContextMenu';
import { isMarkdownPath } from '../../modules/documents/fileGateway';
import { fileStem } from '../../modules/paths';
import { GRAPH_TAB_PATH, isEmptyTabPath } from '../../modules/ui-state';
import { fileActionItems, type FileMenuActions } from './fileActionItems';
import './TitlebarTab.css';

export interface TabFileActions extends FileMenuActions {
  commitRename: (path: string, nextName: string) => void;
  cancelRename: () => void;
}

interface TitlebarTabProps {
  path: string;
  isActive: boolean;
  renaming?: boolean;
  actions?: TabFileActions;
  onSelect?: (path: string) => void;
  onClose?: (path: string) => void;
}

function getTitle(path: string) {
  if (path === GRAPH_TAB_PATH) return t('tabs.graph');
  if (isEmptyTabPath(path)) return t('tabs.newTab');
  return fileStem(path);
}

export const TitlebarTab = memo(function TitlebarTab({
  path,
  isActive,
  renaming = false,
  actions,
  onSelect,
  onClose,
}: TitlebarTabProps) {
  const title = getTitle(path);
  const canFileActions = Boolean(actions) && isMarkdownPath(path);
  const menu = useContextMenu();

  return (
    <div
      className={`q-titlebar-tab ${isActive ? 'active' : ''}`}
      data-tab-path={path}
      data-renaming={renaming ? '' : undefined}
      role="tab"
      tabIndex={0}
      aria-selected={isActive}
      onClick={() => {
        if (!renaming) onSelect?.(path);
      }}
      onKeyDown={(event: KeyboardEvent<HTMLDivElement>) => {
        if (event.target !== event.currentTarget) return;
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault();
          onSelect?.(path);
        }
      }}
      onAuxClick={(event: MouseEvent<HTMLDivElement>) => {
        if (event.button === 1) onClose?.(path);
      }}
      onContextMenu={(event: MouseEvent<HTMLDivElement>) => {
        if (!canFileActions || renaming) return;
        menu.onContextMenu(event);
      }}
    >
      {path === GRAPH_TAB_PATH && (
        <span className="q-titlebar-tab-icon" aria-hidden="true">
          <Icon icon={Network} strokeWidth={1.5} />
        </span>
      )}
      <div className="q-titlebar-tab-content">
        <div className="q-titlebar-tab-spacer" aria-hidden="true">
          {title}
        </div>
        {renaming && actions ? (
          <InlineRenameField
            name={title}
            ariaLabel={t('tabs.renameAria')}
            className="q-titlebar-tab-rename"
            onCommit={(nextName) => actions.commitRename(path, nextName)}
            onCancel={actions.cancelRename}
          />
        ) : (
          <div
            className="q-titlebar-tab-title"
            onMouseEnter={(event) => titleWhenClipped(event.currentTarget, title)}
          >
            {title}
          </div>
        )}
      </div>
      {!renaming && (
        <IconButton
          label={t('tabs.closeNamed', { title })}
          size="small"
          className="q-titlebar-tab-close"
          onClick={(event: MouseEvent<HTMLButtonElement>) => {
            event.stopPropagation();
            onClose?.(path);
          }}
        >
          <Icon icon={X} strokeWidth={1.5} />
        </IconButton>
      )}
      {canFileActions && actions && menu.open && (
        <Menu
          open
          position={menu.position}
          items={fileActionItems(actions, path)}
          onClose={menu.close}
          ariaLabel={t('tabs.actions')}
        />
      )}
    </div>
  );
});
