const LINE_HEIGHT_PIXELS = 16;
const PAGE_HEIGHT_PIXELS = 100;
const PINCH_RESPONSE = 0.01;
const WHEEL_RESPONSE = 0.0018;
const MOUSE_NOTCH_PIXELS = 40;
const GESTURE_GAP_MS = 220;

type WheelGesture =
  | { kind: 'zoom'; factor: number }
  | { kind: 'pan'; dx: number; dy: number };

export interface WheelSample {
  deltaX: number;
  deltaY: number;
  deltaMode: number;
  ctrlKey: boolean;
}

export function wheelStep(deltaMode: number): number {
  if (deltaMode === 1) return LINE_HEIGHT_PIXELS;
  if (deltaMode === 2) return PAGE_HEIGHT_PIXELS;
  return 1;
}

export class WheelGestureReader {
  private glide = false;
  private lastAt = Number.NEGATIVE_INFINITY;

  read(sample: WheelSample, now: number): WheelGesture {
    const continued = now - this.lastAt < GESTURE_GAP_MS;
    this.lastAt = now;
    const step = wheelStep(sample.deltaMode);
    const dx = sample.deltaX * step;
    const dy = sample.deltaY * step;
    const notch = isMouseNotch(dx, dy);
    this.glide = !sample.ctrlKey && !notch && (!continued || this.glide);
    if (this.glide) return { kind: 'pan', dx: -dx, dy: -dy };
    const response = sample.ctrlKey && !notch ? PINCH_RESPONSE : WHEEL_RESPONSE;
    return { kind: 'zoom', factor: Math.exp(-dy * response) };
  }
}

function isMouseNotch(dx: number, dy: number): boolean {
  return dx === 0 && Number.isInteger(dy) && Math.abs(dy) >= MOUSE_NOTCH_PIXELS;
}
