import type { ButtonHTMLAttributes } from 'react';
import './Chip.css';

interface ChipProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  label: string;
  selected?: boolean;
}

export function Chip({ label, selected = false, className = '', ...props }: ChipProps) {
  return (
    <button
      {...props}
      type="button"
      className={`q-chip${selected ? ' q-chip--active' : ''} ${className}`.trim()}
      aria-pressed={selected}
    >
      {label}
    </button>
  );
}
