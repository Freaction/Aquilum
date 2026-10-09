// @vitest-environment happy-dom
import { afterEach, expect, it, vi } from 'vitest';
import { actAndSettle, mountDom, type MountedDom } from '../../testing/mountDom';
import { Menu } from './Menu';

let mounted: MountedDom;
afterEach(() => { mounted?.unmount(); vi.restoreAllMocks(); });

it('moves a bottom-right menu into the viewport and keeps internal scrolling open', async () => {
  vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockReturnValue({ top: 580, left: 790, bottom: 880, right: 990, width: 200, height: 300 } as DOMRect);
  vi.spyOn(window, 'innerHeight', 'get').mockReturnValue(600);
  vi.spyOn(window, 'innerWidth', 'get').mockReturnValue(800);
  const close = vi.fn();
  await actAndSettle(() => { mounted = mountDom(<Menu open position={{ top: 580, left: 790 }} items={[{ id: 'format', label: 'Format table', onSelect: () => {} }]} onClose={close} />); });
  const menu = document.querySelector<HTMLElement>('[role="menu"]')!;
  expect(parseFloat(menu.style.top)).toBeLessThanOrEqual(292);
  expect(parseFloat(menu.style.left)).toBeLessThanOrEqual(592);
  menu.dispatchEvent(new Event('scroll', { bubbles: false }));
  expect(close).not.toHaveBeenCalled();
  window.dispatchEvent(new Event('scroll'));
  expect(close).toHaveBeenCalledOnce();
});

it('caps an oversized menu to the viewport height', async () => {
  vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockReturnValue({ top: 580, left: 20, bottom: 1480, right: 220, width: 200, height: 900 } as DOMRect);
  vi.spyOn(window, 'innerHeight', 'get').mockReturnValue(600);
  await actAndSettle(() => { mounted = mountDom(<Menu open position={{ top: 580, left: 20 }} items={[]} onClose={() => {}} />); });
  const menu = document.querySelector<HTMLElement>('[role="menu"]')!;
  expect(menu.style.top).toBe('8px');
  expect(menu.style.maxHeight).toBe('584px');
  expect(menu.style.overflowY).toBe('auto');
});

it('renders shortcuts separately and caps its content width inside narrow viewports', async () => {
  vi.spyOn(window, 'innerWidth', 'get').mockReturnValue(375);
  vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockReturnValue({ width: 359, height: 80 } as DOMRect);
  await actAndSettle(() => { mounted = mountDom(<Menu open position={{ top: 30, left: 360 }} items={[{ id: 'width', label: 'Full-width text', shortcut: '⌘⌥W', onSelect: () => {} }]} onClose={() => {}} />); });
  const menu = document.querySelector<HTMLElement>('[role="menu"]')!;
  expect(menu.querySelector('.q-menu__item-label')!.textContent).toBe('Full-width text');
  expect(menu.querySelector('.q-menu__item-shortcut')!.textContent).toBe('⌘⌥W');
  expect(menu.style.width).toBe('');
  expect(menu.style.maxWidth).toBe('359px');
  expect(menu.style.left).toBe('8px');
});
