import './Switch.css';

interface SwitchProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  label?: string;
  disabled?: boolean;
}

export function Switch({ checked, onChange, label, disabled }: SwitchProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      disabled={disabled}
      className={`q-switch${checked ? ' q-switch--checked' : ''}`}
      onClick={() => onChange(!checked)}
    >
      <span className="q-switch__handle" />
    </button>
  );
}
