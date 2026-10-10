import { useState } from 'react';
import { Input } from '../../Common/Input';
import { clamp } from '../../../modules/math';
import { t } from '../../../i18n';

interface NumberControlProps {
  value: number;
  min?: number;
  max?: number;
  step?: number;
  stepButtons?: boolean;
  onChange: (value: number) => void;
  onCommit?: () => void;
  ariaLabel?: string;
}

export function NumberControl({ value, min, max, step = 1, stepButtons = false, onChange, onCommit, ariaLabel }: NumberControlProps) {
  const [draft, setDraft] = useState<string | null>(null);
  const bound = (next: number) => clamp(next, min ?? -Infinity, max ?? Infinity);
  const shift = (direction: 1 | -1) => {
    onChange(bound(value + direction * step));
    setDraft(null);
  };

  return (
    <Input
      value={draft ?? value}
      ariaLabel={ariaLabel ?? t('settings.numberValue')}
      stepper={stepButtons ? {
        onStep: shift,
        canDecrease: value > (min ?? -Infinity),
        canIncrease: value < (max ?? Infinity),
      } : undefined}
      onChange={(raw) => {
        setDraft(raw);
        const next = Number(raw.replace(',', '.'));
        if (!Number.isNaN(next) && raw.trim() !== '') onChange(bound(next));
      }}
      onBlur={() => {
        setDraft(null);
        onCommit?.();
      }}
      onKeyDown={(event) => {
        if (event.key === 'Enter' && onCommit) {
          event.preventDefault();
          onCommit();
          return true;
        }
        if (event.key !== 'ArrowUp' && event.key !== 'ArrowDown') return false;
        event.preventDefault();
        shift(event.key === 'ArrowUp' ? 1 : -1);
        return true;
      }}
    />
  );
}
