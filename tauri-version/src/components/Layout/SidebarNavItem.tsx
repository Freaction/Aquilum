import type { ReactNode } from 'react';
import './Sidebar.css';

interface SidebarNavItemProps {
  label: string;
  icon?: ReactNode;
  active?: boolean;
  onClick: () => void;
}

export function SidebarNavItem({
  label,
  icon,
  active = false,
  onClick,
}: SidebarNavItemProps) {
  return (
    <button
      type="button"
      className={`q-file-item ${active ? 'active' : ''}`.trim()}
      aria-current={active ? 'page' : undefined}
      onClick={onClick}
    >
      {icon && (
        <span className="q-file-icon" aria-hidden="true">
          {icon}
        </span>
      )}
      <span className="q-file-name">{label}</span>
    </button>
  );
}
