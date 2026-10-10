import { useSyncExternalStore } from 'react';
import { blockingWheelGesture } from '../wheelGesture';
import { matchesShortcut, SHORTCUTS, ZOOM_IN_ALIASES } from '../../config/shortcuts';
import { clamp } from '../math';

const STORAGE_KEY = 'aquilum-scale-factor';
const ROOT_FONT_SIZE_PX = 16;
export const MIN_SCALE = 0.7;
export const MAX_SCALE = 1.5;
export const SCALE_STEP = 0.05;

let currentScale: number = loadScale();
const listeners = new Set<() => void>();

export function pxToRem(px: number): string {
  return `${px / ROOT_FONT_SIZE_PX}rem`;
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

export function useInterfaceScale(): number {
  return useSyncExternalStore(subscribe, () => currentScale);
}

function loadScale(): number {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved !== null) {
      const value = parseFloat(saved);
      if (!isNaN(value) && value >= MIN_SCALE && value <= MAX_SCALE) {
        return value;
      }
    }
  } catch {
    return 1.0;
  }
  return 1.0;
}

function saveScale(scale: number): void {
  try {
    localStorage.setItem(STORAGE_KEY, scale.toString());
  } catch {
    return;
  }
}

function applyScale(scale: number): void {
  document.documentElement.style.setProperty('--q-base-font-size', `${ROOT_FONT_SIZE_PX * scale}px`);
}

export function setInterfaceScale(next: number): void {
  currentScale = clamp(+next.toFixed(2), MIN_SCALE, MAX_SCALE);
  applyScale(currentScale);
  saveScale(currentScale);
  listeners.forEach((listener) => listener());
}

function hasZoomModifier(event: KeyboardEvent | WheelEvent): boolean {
  return event.ctrlKey || event.metaKey;
}

export function initScaling(): () => void {
  applyScale(currentScale);

  const zoomByWheel = (event: WheelEvent) => {
    if (event.defaultPrevented) return;
    if (!hasZoomModifier(event)) return;
    event.preventDefault();
    setInterfaceScale(currentScale + (event.deltaY < 0 ? SCALE_STEP : -SCALE_STEP));
  };

  const gesture = blockingWheelGesture(window, zoomByWheel);
  const unbindZoomByWheel = () => gesture.disarm();

  const handleKeydown = (event: KeyboardEvent) => {
    if (!hasZoomModifier(event)) return;
    gesture.arm();
    if ([SHORTCUTS.ZOOM_IN, ...ZOOM_IN_ALIASES].some((shortcut) => matchesShortcut(event, shortcut))) {
      event.preventDefault();
      setInterfaceScale(currentScale + SCALE_STEP);
      return;
    }
    if (matchesShortcut(event, SHORTCUTS.ZOOM_OUT)) {
      event.preventDefault();
      setInterfaceScale(currentScale - SCALE_STEP);
      return;
    }
    if (matchesShortcut(event, SHORTCUTS.ZOOM_RESET)) {
      event.preventDefault();
      setInterfaceScale(1.0);
    }
  };

  const handleKeyup = (event: KeyboardEvent) => {
    if (hasZoomModifier(event)) return;
    unbindZoomByWheel();
  };

  const armZoomByWheel = (event: WheelEvent) => {
    if (hasZoomModifier(event)) gesture.arm();
  };

  window.addEventListener('keydown', handleKeydown);
  window.addEventListener('keyup', handleKeyup);
  window.addEventListener('blur', unbindZoomByWheel);
  window.addEventListener('wheel', armZoomByWheel, { passive: true });

  return () => {
    window.removeEventListener('keydown', handleKeydown);
    window.removeEventListener('keyup', handleKeyup);
    window.removeEventListener('blur', unbindZoomByWheel);
    window.removeEventListener('wheel', armZoomByWheel);
    unbindZoomByWheel();
  };
}
