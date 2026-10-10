import { WheelGestureReader } from './wheelGestures';

const DRAG_THRESHOLD_PIXELS = 2;

export interface PointerTarget {
  panBy(dx: number, dy: number): void;
  dragging(active: boolean): void;
  zoomBy(factor: number, paddingBoxX: number, paddingBoxY: number): void;
  pointerAt(paddingBoxX: number, paddingBoxY: number): void;
  select(paddingBoxX: number, paddingBoxY: number): void;
  leave(): void;
  reset(): void;
}

interface Point {
  x: number;
  y: number;
}

interface Spread extends Point {
  radius: number;
}

interface WebKitGestureEvent extends UIEvent {
  scale: number;
  clientX: number;
  clientY: number;
}

export function attachPointerControls(
  canvas: HTMLCanvasElement,
  target: PointerTarget,
): () => void {
  const pointers = new Map<number, Point>();
  let dragged = false;
  let gestureScale = 1;
  const wheelGestures = new WheelGestureReader();

  const paddingBoxOf = (clientX: number, clientY: number): Point => {
    const box = canvas.getBoundingClientRect();
    return { x: clientX - box.left, y: clientY - box.top };
  };

  const startDragging = () => {
    if (dragged) return;
    dragged = true;
    target.dragging(true);
  };

  const handleWheel = (event: WheelEvent) => {
    event.preventDefault();
    event.stopPropagation();
    const gesture = wheelGestures.read(event, event.timeStamp);
    if (gesture.kind === 'pan') {
      target.panBy(gesture.dx, gesture.dy);
      return;
    }
    target.zoomBy(gesture.factor, event.offsetX, event.offsetY);
  };

  const handlePointerDown = (event: PointerEvent) => {
    if (event.button !== 0) return;
    canvas.setPointerCapture(event.pointerId);
    pointers.set(event.pointerId, { x: event.clientX, y: event.clientY });
    if (pointers.size === 1) dragged = false;
    else startDragging();
  };

  const handlePointerMove = (event: PointerEvent) => {
    if (!pointers.has(event.pointerId)) {
      target.pointerAt(event.offsetX, event.offsetY);
      return;
    }
    const before = spreadOf(pointers);
    pointers.set(event.pointerId, { x: event.clientX, y: event.clientY });
    const after = spreadOf(pointers);
    const dx = after.x - before.x;
    const dy = after.y - before.y;
    if (Math.abs(dx) + Math.abs(dy) > DRAG_THRESHOLD_PIXELS) startDragging();
    target.panBy(dx, dy);
    if (pointers.size < 2 || before.radius === 0) return;
    const anchor = paddingBoxOf(after.x, after.y);
    target.zoomBy(after.radius / before.radius, anchor.x, anchor.y);
  };

  const handlePointerUp = (event: PointerEvent) => {
    if (!pointers.delete(event.pointerId)) return;
    canvas.releasePointerCapture(event.pointerId);
    if (pointers.size > 0) return;
    if (dragged) {
      target.dragging(false);
      return;
    }
    target.select(event.offsetX, event.offsetY);
  };

  const handlePointerCancel = (event: PointerEvent) => {
    if (!pointers.delete(event.pointerId)) return;
    if (pointers.size === 0 && dragged) target.dragging(false);
  };

  const handleGestureStart = (event: Event) => {
    event.preventDefault();
    gestureScale = 1;
  };

  const handleGestureChange = (event: Event) => {
    event.preventDefault();
    const gesture = event as WebKitGestureEvent;
    const anchor = paddingBoxOf(gesture.clientX, gesture.clientY);
    target.zoomBy(gesture.scale / gestureScale, anchor.x, anchor.y);
    gestureScale = gesture.scale;
  };

  const handlePointerLeave = () => target.leave();
  const handleDoubleClick = () => target.reset();

  const listeners: [string, EventListener, AddEventListenerOptions?][] = [
    ['wheel', handleWheel as EventListener, { passive: false }],
    ['pointerdown', handlePointerDown as EventListener],
    ['pointermove', handlePointerMove as EventListener],
    ['pointerup', handlePointerUp as EventListener],
    ['pointercancel', handlePointerCancel as EventListener],
    ['pointerleave', handlePointerLeave],
    ['dblclick', handleDoubleClick],
    ['gesturestart', handleGestureStart],
    ['gesturechange', handleGestureChange],
  ];
  for (const [type, listener, options] of listeners) {
    canvas.addEventListener(type, listener, options);
  }
  return () => {
    for (const [type, listener] of listeners) canvas.removeEventListener(type, listener);
  };
}

function spreadOf(points: Map<number, Point>): Spread {
  let x = 0;
  let y = 0;
  for (const point of points.values()) {
    x += point.x;
    y += point.y;
  }
  x /= points.size;
  y /= points.size;
  let radius = 0;
  for (const point of points.values()) radius += Math.hypot(point.x - x, point.y - y);
  return { x, y, radius: radius / points.size };
}
