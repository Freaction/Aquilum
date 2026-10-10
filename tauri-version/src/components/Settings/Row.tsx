import type { ReactNode } from 'react';
import './Row.css';

interface RowProps {
  label: string;
  description?: string;
  children?: ReactNode;
}

export function Row({ label, description, children }: RowProps) {
  return (
    <div className="q-settings-row">
      <div className="q-settings-row__text">
        <span className="q-settings-row__label">{label}</span>
        {description ? (
          <span className="q-settings-row__description">{description}</span>
        ) : null}
      </div>
      <div className="q-settings-row__swap">{children}</div>
    </div>
  );
}
