import { ChevronsUpDown, Settings } from 'lucide';
import { Icon } from '../Common/Icon';
import { IconButton } from '../Common/IconButton';
import { t } from '../../i18n';

interface SidebarFooterProps {
  workspaceName: string;
  workspacePath: string | null;
  onOpenSettings: () => void;
  onOpenWorkspaces: () => void;
}

export function SidebarFooter({
  workspaceName,
  workspacePath,
  onOpenSettings,
  onOpenWorkspaces,
}: SidebarFooterProps) {
  return (
    <div className="q-sidebar-footer">
      <button
        type="button"
        className="q-file-item q-sidebar-footer-tab"
        onClick={onOpenWorkspaces}
        title={t('workspace.switch')}
      >
        <span className="q-file-icon" aria-hidden="true">
          <Icon icon={ChevronsUpDown} />
        </span>
        <span className="q-sidebar-footer-title" title={workspacePath || ''}>
          {workspaceName}
        </span>
      </button>
      <IconButton label={t('common.settings')} onClick={onOpenSettings}>
        <Icon icon={Settings} />
      </IconButton>
    </div>
  );
}
