import { useEffect, useRef, useState } from 'react';
import { useStableCallback } from '../../../hooks/useStableCallback';
import './ColorControl.css';

interface ColorControlProps {
  value: string;
  onChange: (value: string) => void;
  ariaLabel?: string;
}

function parseHex(raw: string): string | null {
  const match = /^#?([0-9a-f]{6})$/i.exec(raw.trim());
  return match ? `#${match[1].toLowerCase()}` : null;
}

export function ColorControl({ value, onChange, ariaLabel }: ColorControlProps) {
  const [hex, setHex] = useState(value);
  const timer = useRef<ReturnType<typeof setTimeout>>(undefined);
  const pending = useRef<string | null>(null);
  const color = parseHex(hex) ?? value;

  const flush = useStableCallback(() => {
    clearTimeout(timer.current);
    const next = pending.current;
    pending.current = null;
    if (next !== null) onChange(next);
  });

  useEffect(() => setHex(value), [value]);
  useEffect(() => flush, [flush]);

  const schedule = (next: string) => {
    pending.current = next;
    clearTimeout(timer.current);
    timer.current = setTimeout(flush, 200);
  };

  return (
    <div className="q-color-control">
      <span className="q-color-control__swatch" style={{ background: color }}>
        <input
          type="color"
          className="q-color-control__native"
          value={color}
          aria-label={ariaLabel}
          onChange={(event) => {
            setHex(event.currentTarget.value);
            schedule(event.currentTarget.value);
          }}
        />
      </span>
      <input
        className="q-color-control__hex"
        value={hex}
        aria-label={ariaLabel}
        onChange={(event) => {
          setHex(event.currentTarget.value);
          const next = parseHex(event.currentTarget.value);
          if (next) schedule(next);
        }}
        onBlur={() => setHex(value)}
      />
    </div>
  );
}
