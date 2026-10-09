import { useEffect, useLayoutEffect, useRef, useState, type RefObject } from 'react';
import { createPortal } from 'react-dom';
import { Check, ChevronRight, type IconNode } from 'lucide';
import { Icon } from './Icon';
import './Menu.css';

export interface MenuItem {
  id: string;
  label: string;
  shortcut?: string;
  disabled?: boolean;
  checked?: boolean;
  icon?: IconNode;
  /** Вложенные пункты: открываются справа при наведении или клике. */
  children?: MenuItem[];
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
  const subRef = useRef<HTMLDivElement>(null);
  const [sub, setSub] = useState<{ id: string; anchor: DOMRect; focus: boolean } | null>(null);
  const subItems = sub ? items.find(item => item.id === sub.id)?.children ?? [] : [];

  useEffect(() => { if (!open) setSub(null); }, [open, items]);

  useLayoutEffect(() => {
    const menu = subRef.current;
    if (!sub || !menu) return;
    const rect = menu.getBoundingClientRect();
    // Справа от пункта; если не помещается — слева от него.
    const right = sub.anchor.right + VIEWPORT_MARGIN_PX / 2;
    const left = right + rect.width > window.innerWidth - VIEWPORT_MARGIN_PX ? sub.anchor.left - rect.width - VIEWPORT_MARGIN_PX / 2 : right;
    menu.style.left = `${Math.max(VIEWPORT_MARGIN_PX, left)}px`;
    menu.style.top = `${Math.max(VIEWPORT_MARGIN_PX, Math.min(sub.anchor.top, window.innerHeight - rect.height - VIEWPORT_MARGIN_PX))}px`;
    if (sub.focus) menu.querySelector<HTMLButtonElement>('button:not(:disabled)')?.focus();
  }, [sub]);

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
      {items.map((item) => {
        const nested = Boolean(item.children?.length);
        const openSub = (event: { currentTarget: HTMLElement }, focus: boolean) => {
          setSub({ id: item.id, anchor: event.currentTarget.getBoundingClientRect(), focus });
        };
        return (
          <button
            key={item.id}
            type="button"
            role={role === 'listbox' ? 'option' : 'menuitem'}
            aria-selected={role === 'listbox' ? Boolean(item.checked) : undefined}
            aria-haspopup={nested ? 'menu' : undefined}
            aria-expanded={nested ? sub?.id === item.id : undefined}
            disabled={item.disabled}
            className="q-menu__item"
            onMouseEnter={(event) => { if (nested && !item.disabled) openSub(event, false); else setSub(null); }}
            onKeyDown={(event) => { if (nested && event.key === 'ArrowRight') { event.preventDefault(); openSub(event, true); } }}
            onClick={(event) => {
              if (item.disabled) return;
              if (nested) { openSub(event, true); return; }
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
            {nested && <Icon icon={ChevronRight} className="q-menu__item-submenu" />}
          </button>
        );
      })}
      {sub && subItems.length > 0 && (
        <div
          ref={subRef}
          role="menu"
          aria-label={items.find(item => item.id === sub.id)?.label}
          className="q-menu q-menu--sub"
          style={{ top: sub.anchor.top, left: sub.anchor.right }}
          onKeyDown={(event) => {
            if (event.key !== 'ArrowLeft') return;
            event.preventDefault();
            const parent = sub.id;
            setSub(null);
            menuRef.current?.querySelectorAll<HTMLButtonElement>(':scope > .q-menu__item')[items.findIndex(item => item.id === parent)]?.focus();
          }}
        >
          {subItems.map(child => (
            <button
              key={child.id}
              type="button"
              role="menuitem"
              disabled={child.disabled}
              className="q-menu__item"
              onClick={() => {
                if (child.disabled) return;
                child.onSelect();
                onClose();
              }}
            >
              <span className="q-menu__item-label">{child.label}</span>
            </button>
          ))}
        </div>
      )}
    </div>,
    document.body,
  );
}
