import { useEffect, type RefObject } from 'react';
import { clamp } from '../../modules/math';
import { blockingWheelGesture } from '../../modules/wheelGesture';

const LINE_HEIGHT_PX = 16;

function wheelDeltaPx(event: WheelEvent): number {
  const delta = Math.abs(event.deltaX) > Math.abs(event.deltaY) ? event.deltaX : event.deltaY;
  return event.deltaMode === WheelEvent.DOM_DELTA_LINE ? delta * LINE_HEIGHT_PX : delta;
}

export function useHorizontalWheelScroll(
  viewportRef: RefObject<HTMLDivElement | null>,
  enabled: boolean,
  resetKey: unknown,
): void {
  useEffect(() => {
    const viewport = viewportRef.current;
    if (!viewport || !enabled) return;

    const redirectWheel = (event: WheelEvent) => {
      if (event.ctrlKey) return;
      const delta = wheelDeltaPx(event);
      if (delta === 0) return;
      const range = viewport.scrollWidth - viewport.clientWidth;
      if (range <= 0) return;
      event.preventDefault();
      viewport.scrollLeft = clamp(viewport.scrollLeft + delta, 0, range);
    };

    const gesture = blockingWheelGesture(viewport, redirectWheel);
    const arm = () => gesture.arm();
    const disarm = () => gesture.disarm();

    viewport.addEventListener('pointerenter', arm);
    viewport.addEventListener('pointerleave', disarm);
    viewport.addEventListener('wheel', arm, { passive: true });

    return () => {
      viewport.removeEventListener('pointerenter', arm);
      viewport.removeEventListener('pointerleave', disarm);
      viewport.removeEventListener('wheel', arm);
      gesture.disarm();
    };
  }, [enabled, resetKey, viewportRef]);
}
