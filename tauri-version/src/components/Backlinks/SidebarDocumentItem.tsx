import type { ButtonHTMLAttributes } from 'react';
import './SidebarDocumentItem.css';

interface SidebarDocumentItemProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  label: string;
  counter?: string;
}

export function SidebarDocumentItem({
  label,
  counter,
  className = '',
  title = label,
  type = 'button',
  ...props
}: SidebarDocumentItemProps) {
  return (
    <button
      {...props}
      type={type}
      className={`q-sidebar-document-item ${className}`.trim()}
      title={title}
    >
      <span className="q-sidebar-document-item__title">{label}</span>
      {counter !== undefined && (
        <span className="q-panel-counter">{counter}</span>
      )}
    </button>
  );
}
