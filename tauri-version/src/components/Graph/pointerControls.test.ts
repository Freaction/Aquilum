import { describe, expect, it, vi } from 'vitest';
import { attachPointerControls, type PointerTarget } from './pointerControls';

interface MockCanvas {
  canvas: HTMLCanvasElement;
  emit(type: string, event: unknown): void;
}

function createMockCanvas(): MockCanvas {
  const listeners = new Map<string, ((event: unknown) => void)[]>();
  const canvas = {
    addEventListener: vi.fn((type: string, listener: (event: unknown) => void) => {
      const list = listeners.get(type) ?? [];
      list.push(listener);
      listeners.set(type, list);
    }),
    removeEventListener: vi.fn((type: string, listener: (event: unknown) => void) => {
      const list = listeners.get(type);
      if (!list) return;
      listeners.set(type, list.filter((item) => item !== listener));
    }),
    setPointerCapture: vi.fn(),
    releasePointerCapture: vi.fn(),
    getBoundingClientRect: () => ({ left: 10, top: 20 }),
  } as unknown as HTMLCanvasElement;

  return {
    canvas,
    emit(type: string, event: unknown) {
      for (const listener of listeners.get(type) ?? []) {
        listener(event);
      }
    },
  };
}

function createMockTarget(): PointerTarget {
  return {
    panBy: vi.fn(),
    dragging: vi.fn(),
    zoomBy: vi.fn(),
    pointerAt: vi.fn(),
    select: vi.fn(),
    leave: vi.fn(),
    reset: vi.fn(),
  };
}

describe('attachPointerControls', () => {
  it('handles trackpad pinch as a zoom event', () => {
    const { canvas, emit } = createMockCanvas();
    const target = createMockTarget();
    const detach = attachPointerControls(canvas, target);

    let prevented = false;
    let stopped = false;
    emit('wheel', {
      offsetX: 100,
      offsetY: 150,
      deltaX: 0,
      deltaY: -10,
      deltaMode: 0,
      ctrlKey: true,
      metaKey: false,
      timeStamp: 100,
      preventDefault: () => {
        prevented = true;
      },
      stopPropagation: () => {
        stopped = true;
      },
    });

    expect(target.zoomBy).toHaveBeenCalledTimes(1);
    const [factor, x, y] = (target.zoomBy as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(factor).toBeGreaterThan(1);
    expect(x).toBe(100);
    expect(y).toBe(150);
    expect(prevented).toBe(true);
    expect(stopped).toBe(true);

    detach();
  });

  it('handles mouse drag as pan', () => {
    const { canvas, emit } = createMockCanvas();
    const target = createMockTarget();
    const detach = attachPointerControls(canvas, target);

    emit('pointerdown', {
      pointerId: 1,
      clientX: 50,
      clientY: 50,
      button: 0,
      pointerType: 'mouse',
    });

    emit('pointermove', {
      pointerId: 1,
      clientX: 60,
      clientY: 70,
      button: 0,
      pointerType: 'mouse',
    });

    expect(target.dragging).toHaveBeenCalledWith(true);
    expect(target.panBy).toHaveBeenCalledWith(10, 20);

    emit('pointerup', {
      pointerId: 1,
      clientX: 60,
      clientY: 70,
      button: 0,
      pointerType: 'mouse',
    });

    expect(target.dragging).toHaveBeenCalledWith(false);
    expect(target.select).not.toHaveBeenCalled();

    detach();
  });

  it('selects a node on single click without drag', () => {
    const { canvas, emit } = createMockCanvas();
    const target = createMockTarget();
    const detach = attachPointerControls(canvas, target);

    emit('pointerdown', {
      pointerId: 1,
      clientX: 50,
      clientY: 60,
      offsetX: 50,
      offsetY: 60,
      button: 0,
      pointerType: 'mouse',
    });

    emit('pointerup', {
      pointerId: 1,
      clientX: 50,
      clientY: 60,
      offsetX: 50,
      offsetY: 60,
      button: 0,
      pointerType: 'mouse',
    });

    expect(target.select).toHaveBeenCalledWith(50, 60);

    detach();
  });

  it('zooms around the midpoint when two fingers spread apart', () => {
    const { canvas, emit } = createMockCanvas();
    const target = createMockTarget();
    const detach = attachPointerControls(canvas, target);
    const touch = (pointerId: number, clientX: number) => ({
      pointerId, clientX, clientY: 120, offsetX: clientX - 10, offsetY: 100, button: 0, pointerType: 'touch',
    });

    emit('pointerdown', touch(1, 100));
    emit('pointerdown', touch(2, 200));
    emit('pointermove', touch(2, 300));
    emit('pointerup', touch(1, 100));
    emit('pointerup', touch(2, 300));

    const [factor, x, y] = (target.zoomBy as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(factor).toBe(2);
    expect(x).toBe(190);
    expect(y).toBe(100);
    expect(target.panBy).toHaveBeenCalledWith(50, 0);
    expect(target.select).not.toHaveBeenCalled();
    expect(target.dragging).toHaveBeenLastCalledWith(false);

    detach();
  });
});
