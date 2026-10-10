import { createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { Menu, type MenuItem } from './Menu';

let shown: { root: ReturnType<typeof createRoot>; host: HTMLElement } | null = null;

export function closeImperativeMenu(): void {
  if (!shown) return;
  const { root, host } = shown;
  shown = null;
  queueMicrotask(() => {
    root.unmount();
    host.remove();
  });
}

export function showImperativeMenu(x: number, y: number, items: MenuItem[], ariaLabel: string): void {
  closeImperativeMenu();
  const host = document.createElement('div');
  document.body.appendChild(host);
  const root = createRoot(host);
  shown = { root, host };
  root.render(createElement(Menu, {
    open: true,
    position: { top: y, left: x },
    items,
    ariaLabel,
    onClose: closeImperativeMenu,
  }));
}
