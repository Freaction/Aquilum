import { useEffect, useLayoutEffect, useRef, type RefObject } from 'react';
import { createPortal } from 'react-dom';
import { Check, type IconNode } from 'lucide';
import { Icon } from './Icon';
import './Menu.css';

export interface MenuItem {
  id: string;
  label: string;
  shortcut?: string;
  disabled?: boolean;
  checked?: boolean;
  icon?: IconNode;
  onSelect: () => void;
}

export interface MenuPosition {
  top: number;
  left: number;
  width?: number;
}

interface MenuProps {
  id?: string;
  open: boolean;
  position: MenuPosition | null;
  items: MenuItem[];
  onClose: () => void;
  role?: 'listbox' | 'menu';
  ariaLabel?: string;
  showCheck?: boolean;
  excludeRef?: RefObject<HTMLElement | null>;
}

const VIEWPORT_MARGIN_PX = 8;

export function Menu({
  id,
  open,
  position,
  items,
  onClose,
  role = 'menu',
  ariaLabel,
  showCheck = false,
  excludeRef,
}: MenuProps) {
  const menuRef = useRef<HTMLDivElement>(null);

  useLayoutEffect(() => {
    const menu = menuRef.current;
    if (!open || !position || !menu) return;
    const height = Math.max(0, window.innerHeight - 2 * VIEWPORT_MARGIN_PX);
    menu.style.maxWidth = `${Math.max(0, window.innerWidth - 2 * VIEWPORT_MARGIN_PX)}px`;
    menu.style.maxHeight = `${height}px`;
    menu.style.overflowY = 'auto';
    const rect = menu.getBoundingClientRect();
    menu.style.left = `${Math.max(VIEWPORT_MARGIN_PX, Math.min(position.left, window.innerWidth - rect.width - VIEWPORT_MARGIN_PX))}px`;
    menu.style.top = `${Math.max(VIEWPORT_MARGIN_PX, Math.min(position.top, window.innerHeight - rect.height - VIEWPORT_MARGIN_PX))}px`;
  }, [open, position, items]);

  useEffect(() => {
    if (!open) return;

    const handlePointerDown = (event: MouseEvent) => {
      const target = event.target as Node;
      if (menuRef.current?.contains(target) || excludeRef?.current?.contains(target)) return;
      onClose();
    };

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') return;
      event.stopPropagation();
      onClose();
    };

    const handleReposition = (event: Event) => {
      if (event.target instanceof Node && menuRef.current?.contains(event.target)) return;
      onClose();
    };

    window.addEventListener('mousedown', handlePointerDown);
    window.addEventListener('keydown', handleKeyDown, true);
    window.addEventListener('resize', handleReposition);
    window.addEventListener('scroll', handleReposition, true);

    return () => {
      window.removeEventListener('mousedown', handlePointerDown);
      window.removeEventListener('keydown', handleKeyDown, true);
      window.removeEventListener('resize', handleReposition);
      window.removeEventListener('scroll', handleReposition, true);
    };
  }, [excludeRef, onClose, open]);

  if (!open || !position) return null;

  return createPortal(
    <div
      ref={menuRef}
      id={id}
      role={role}
      aria-label={ariaLabel}
      className="q-menu"
      style={{
        top: position.top,
        left: position.left,
        width: position.width,
      }}
    >
      {items.map((item) => (
        <button
          key={item.id}
          type="button"
          role={role === 'listbox' ? 'option' : 'menuitem'}
          aria-selected={role === 'listbox' ? Boolean(item.checked) : undefined}
          disabled={item.disabled}
          className="q-menu__item"
          onClick={() => {
            if (item.disabled) return;
            item.onSelect();
            onClose();
          }}
        >
          {item.icon && (
            <span className="q-menu__item-icon" aria-hidden="true">
              <Icon icon={item.icon} strokeWidth={1.2} />
            </span>
          )}
          <span className="q-menu__item-label">{item.label}</span>
          {item.shortcut && <kbd className="q-menu__item-shortcut">{item.shortcut}</kbd>}
          {showCheck && item.checked && (
            <Icon icon={Check} className="q-menu__item-check" />
          )}
        </button>
      ))}
    </div>,
    document.body,
  );
}
