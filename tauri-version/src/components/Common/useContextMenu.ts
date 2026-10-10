import { useCallback, useState, type MouseEvent } from 'react';
import type { MenuPosition } from './Menu';

export function useContextMenu() {
  const [open, setOpen] = useState(false);
  const [position, setPosition] = useState<MenuPosition | null>(null);

  const close = useCallback(() => {
    setOpen(false);
    setPosition(null);
  }, []);

  const onContextMenu = useCallback((event: MouseEvent<Element>) => {
    event.preventDefault();
    event.stopPropagation();
    setPosition({ top: event.clientY, left: event.clientX });
    setOpen(true);
  }, []);

  return { open, position, onContextMenu, close };
}
