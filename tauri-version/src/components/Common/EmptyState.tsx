import type { IconNode } from 'lucide';
import { Icon } from './Icon';
import type { ReactNode } from 'react';
import './EmptyState.css';

interface EmptyStateProps {
  icon: IconNode;
  title: string;
  description?: ReactNode;
  children?: ReactNode;
  compact?: boolean;
}

export function EmptyState({
  icon,
  title,
  description,
  children,
  compact = false,
}: EmptyStateProps) {
  return (
    <div className={`q-empty-state ${compact ? 'q-empty-state--compact' : ''}`.trim()}>
      <span className="q-empty-state__icon" aria-hidden="true">
        <Icon icon={icon} />
      </span>
      <div className="q-empty-state__copy">
        <p className="q-empty-state__title">{title}</p>
        {description && <p className="q-empty-state__description">{description}</p>}
      </div>
      {children && <div className="q-empty-state__actions">{children}</div>}
    </div>
  );
}
