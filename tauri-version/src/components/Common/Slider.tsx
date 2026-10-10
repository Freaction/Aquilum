import type { ChangeEvent } from 'react';
import './Slider.css';

interface SliderProps {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  hint?: string;
  onChange: (value: number) => void;
}

export function Slider({ label, value, min, max, step, hint, onChange }: SliderProps) {
  const handleChange = (event: ChangeEvent<HTMLInputElement>) => {
    onChange(Number(event.currentTarget.value));
  };

  return (
    <label className="q-slider">
      <span className="q-slider-head">
        <span className="q-slider-label">{label}</span>
        {hint && <span className="q-slider-hint">{hint}</span>}
      </span>
      <input
        className="q-slider-input"
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={handleChange}
      />
    </label>
  );
}
