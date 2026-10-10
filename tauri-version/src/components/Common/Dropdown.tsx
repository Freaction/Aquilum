import {
  useCallback,
  useId,
  useLayoutEffect,
  useRef,
  useState,
} from 'react';
import { ChevronDown } from 'lucide';
import { Icon } from './Icon';
import { Menu, type MenuItem } from './Menu';
import './Dropdown.css';

interface DropdownOption {
  value: string;
  label: string;
}

interface DropdownProps {
  value: string;
  options: DropdownOption[];
  onChange: (value: string) => void;
  ariaLabel: string;
  disabled?: boolean;
}

export function Dropdown({
  value,
  options,
  onChange,
  ariaLabel,
  disabled = false,
}: DropdownProps) {
  const listboxId = useId();
  const rootRef = useRef<HTMLDivElement>(null);
  const [open, setOpen] = useState(false);
  const [menuStyle, setMenuStyle] = useState<{ top: number; left: number; width: number } | null>(null);

  const selected = options.find((option) => option.value === value);
  const close = useCallback(() => setOpen(false), []);

  const updateMenuPosition = useCallback(() => {
    const trigger = rootRef.current;
    if (!trigger) return;
    const rect = trigger.getBoundingClientRect();
    setMenuStyle({
      top: rect.bottom + 4,
      left: rect.left,
      width: rect.width,
    });
  }, []);

  useLayoutEffect(() => {
    if (!open) {
      setMenuStyle(null);
      return;
    }
    updateMenuPosition();
  }, [open, updateMenuPosition, value]);

  const items: MenuItem[] = options.map((option) => ({
    id: option.value,
    label: option.label,
    checked: option.value === value,
    onSelect: () => onChange(option.value),
  }));

  return (
    <div ref={rootRef} className={`q-dropdown${open ? ' q-dropdown--open' : ''}${disabled ? ' q-dropdown--disabled' : ''}`}>
      <button
        type="button"
        className="q-dropdown__trigger"
        aria-label={ariaLabel}
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-controls={open ? listboxId : undefined}
        disabled={disabled}
        onClick={() => {
          if (disabled) return;
          setOpen((current) => !current);
        }}
      >
        <span className="q-dropdown__label">{selected?.label ?? value}</span>
        <Icon icon={ChevronDown} className="q-dropdown__chevron" strokeWidth={1.2} />
      </button>
      <Menu
        id={listboxId}
        open={open}
        position={menuStyle}
        items={items}
        onClose={close}
        role="listbox"
        ariaLabel={ariaLabel}
        showCheck
        excludeRef={rootRef}
      />
    </div>
  );
}
