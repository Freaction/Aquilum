import { describe, expect, it } from 'vitest';
import { wheelStep, WheelGestureReader, type WheelSample } from './wheelGestures';

function sample(patch: Partial<WheelSample>): WheelSample {
  return { deltaX: 0, deltaY: 0, deltaMode: 0, ctrlKey: false, ...patch };
}

describe('wheelStep', () => {
  it('normalises line and page wheel modes to pixels', () => {
    expect(wheelStep(0)).toBe(1);
    expect(wheelStep(1)).toBeGreaterThan(1);
    expect(wheelStep(2)).toBeGreaterThan(wheelStep(1));
  });
});

describe('WheelGestureReader', () => {
  it('reads a trackpad pinch as a zoom around the pointer', () => {
    const reader = new WheelGestureReader();

    const inward = reader.read(sample({ deltaY: -10, ctrlKey: true }), 0);
    const outward = reader.read(sample({ deltaY: 10, ctrlKey: true }), 16);

    expect(inward).toEqual({ kind: 'zoom', factor: Math.exp(0.1) });
    expect(outward).toEqual({ kind: 'zoom', factor: Math.exp(-0.1) });
  });

  it('reads mouse wheel notch with ctrlKey using wheel response', () => {
    const reader = new WheelGestureReader();

    const gesture = reader.read(sample({ deltaY: 100, ctrlKey: true }), 0);

    expect(gesture).toEqual({ kind: 'zoom', factor: Math.exp(-100 * 0.0018) });
  });

  it('reads a mouse wheel notch as a zoom', () => {
    const reader = new WheelGestureReader();

    const gesture = reader.read(sample({ deltaY: 100 }), 0);

    expect(gesture.kind).toBe('zoom');
    expect(gesture.kind === 'zoom' && gesture.factor).toBeLessThan(1);
  });

  it('reads a line mode wheel as a zoom', () => {
    const reader = new WheelGestureReader();

    const gesture = reader.read(sample({ deltaY: -3, deltaMode: 1 }), 0);

    expect(gesture.kind).toBe('zoom');
    expect(gesture.kind === 'zoom' && gesture.factor).toBeGreaterThan(1);
  });

  it('reads a two finger glide as a pan that follows the fingers', () => {
    const reader = new WheelGestureReader();

    const gesture = reader.read(sample({ deltaX: 12, deltaY: -8 }), 0);

    expect(gesture).toEqual({ kind: 'pan', dx: -12, dy: 8 });
  });

  it('keeps panning while a glide slows down to tiny vertical steps', () => {
    const reader = new WheelGestureReader();

    reader.read(sample({ deltaX: 9, deltaY: 20 }), 0);
    const tail = reader.read(sample({ deltaY: 2 }), 40);

    expect(tail.kind).toBe('pan');
  });

  it('starts a fresh gesture after a pause', () => {
    const reader = new WheelGestureReader();

    reader.read(sample({ deltaX: 9, deltaY: 20 }), 0);
    const later = reader.read(sample({ deltaY: 100 }), 900);

    expect(later.kind).toBe('zoom');
  });

  it('lets a mouse notch interrupt a glide', () => {
    const reader = new WheelGestureReader();

    reader.read(sample({ deltaX: 9, deltaY: 20 }), 0);
    const notch = reader.read(sample({ deltaY: -100 }), 30);
    const next = reader.read(sample({ deltaY: -100 }), 60);

    expect(notch.kind).toBe('zoom');
    expect(next.kind).toBe('zoom');
  });

  it('never turns a two finger glide into a zoom', () => {
    const reader = new WheelGestureReader();

    const steep = reader.read(sample({ deltaX: 2, deltaY: -20 }), 0);
    const continued = reader.read(sample({ deltaX: 0, deltaY: -30 }), 16);

    expect(steep.kind).toBe('pan');
    expect(continued.kind).toBe('pan');
  });

  it('reads a pinch in the middle of a glide as a zoom', () => {
    const reader = new WheelGestureReader();

    reader.read(sample({ deltaX: 4, deltaY: 12 }), 0);
    const pinch = reader.read(sample({ deltaY: -10, ctrlKey: true }), 16);

    expect(pinch).toEqual({ kind: 'zoom', factor: Math.exp(0.1) });
  });

  it('scales the zoom with how far the wheel turned', () => {
    const slow = new WheelGestureReader().read(sample({ deltaY: -100 }), 0);
    const fast = new WheelGestureReader().read(sample({ deltaY: -300 }), 0);

    expect(slow.kind === 'zoom' && fast.kind === 'zoom' && fast.factor).toBeGreaterThan(
      slow.kind === 'zoom' ? slow.factor : 0,
    );
  });
});
