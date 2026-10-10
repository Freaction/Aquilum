import type { ReactNode } from 'react';
import '../../styles/components/segmented-control.css';

interface SegmentedControlOption {
  label: ReactNode;
  value: string;
}

interface SegmentedControlProps {
  options: SegmentedControlOption[];
  value: string;
  onChange: (value: string) => void;
  stretch?: boolean;
  className?: string;
}

export function SegmentedControl({ options, value, onChange, stretch = false, className = '' }: SegmentedControlProps) {
  const stretched = stretch ? ' q-segmented-control--stretch' : '';
  return (
    <div className={`q-segmented-control${stretched} ${className}`.trim()}>
      {options.map((option) => {
        const active = value === option.value;
        return (
          <button
            key={option.value}
            type="button"
            aria-pressed={active}
            className={`q-segmented-control__button${active ? ' q-segmented-control__button--active' : ''}`}
            onClick={() => onChange(option.value)}
          >
            {option.label}
          </button>
        );
      })}
    </div>
  );
}
