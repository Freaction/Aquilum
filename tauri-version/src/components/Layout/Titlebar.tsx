import { useRef } from 'react';
import { PanelRight, Plus } from 'lucide';
import { Icon } from '../Common/Icon';
import { IconButton } from '../Common/IconButton';
import { useHorizontalWheelScroll } from '../Common/useHorizontalWheelScroll';
import { t } from '../../i18n';
import { isMacOs } from '../../modules/platform';
import { TitlebarTab, type TabFileActions } from './TitlebarTab';
import { useTabStrip } from './useTabStrip';
import './Titlebar.css';

interface TitlebarProps {
  activeFile?: string | null;
  openFiles?: string[];
  renamingPath?: string | null;
  tabActions?: TabFileActions;
  onSelect?: (path: string) => void;
  onClose?: (path: string) => void;
  onNewTab?: () => void;
  onReorder?: (from: number, to: number) => void;
  rightSidebarOpen?: boolean;
  onToggleRightSidebar?: () => void;
}

export function Titlebar({
  activeFile,
  openFiles = [],
  renamingPath = null,
  tabActions,
  onSelect,
  onClose,
  onNewTab,
  onReorder,
  rightSidebarOpen = true,
  onToggleRightSidebar,
}: TitlebarProps) {
  const tabsRef = useRef<HTMLDivElement | null>(null);
  useTabStrip(tabsRef, (from, to) => onReorder?.(from, to));
  useHorizontalWheelScroll(tabsRef, true, null);

  const inset = !isMacOs() && !rightSidebarOpen ? 'q-titlebar--trailing-inset' : '';

  return (
    <div data-tauri-drag-region className={`q-titlebar ${inset}`.trim()}>
      <div className="q-titlebar-tabs" ref={tabsRef}>
        {openFiles.map((path) => (
          <TitlebarTab
            key={path}
            path={path}
            isActive={path === activeFile}
            renaming={renamingPath === path}
            actions={tabActions}
            onSelect={onSelect}
            onClose={onClose}
          />
        ))}
      </div>

      <IconButton
        label={t('tabs.newTab')}
        size="medium"
        className="q-titlebar-new-tab"
        onClick={onNewTab}
      >
        <Icon icon={Plus} strokeWidth={1.5} />
      </IconButton>

      <div className="q-titlebar-menu-wrapper">
        {onToggleRightSidebar && (
          <IconButton
            label={rightSidebarOpen ? t('titlebar.hideRightSidebar') : t('titlebar.showRightSidebar')}
            size="medium"
            aria-expanded={rightSidebarOpen}
            onClick={onToggleRightSidebar}
          >
            <Icon icon={PanelRight} strokeWidth={1.5} />
          </IconButton>
        )}
      </div>
    </div>
  );
}

