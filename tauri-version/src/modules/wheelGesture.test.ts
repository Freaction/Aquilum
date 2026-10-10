import { describe, expect, it, vi } from 'vitest';
import { blockingWheelGesture } from './wheelGesture';

function target() {
  const added: Array<{ type: string; options: unknown }> = [];
  const removed: string[] = [];
  return {
    added,
    removed,
    addEventListener: vi.fn((type: string, _listener: unknown, options: unknown) => {
      added.push({ type, options });
    }),
    removeEventListener: vi.fn((type: string) => {
      removed.push(type);
    }),
    dispatchEvent: () => true,
  } as unknown as EventTarget & { added: Array<{ type: string; options: unknown }>; removed: string[] };
}

describe('blockingWheelGesture', () => {
  it('registers a non-passive wheel listener only once armed', () => {
    const node = target();
    const gesture = blockingWheelGesture(node, () => {});

    expect(node.added).toHaveLength(0);

    gesture.arm();
    expect(node.added).toEqual([{ type: 'wheel', options: { passive: false } }]);

    gesture.arm();
    expect(node.added).toHaveLength(1);
  });

  it('removes the listener on disarm and ignores repeated disarms', () => {
    const node = target();
    const gesture = blockingWheelGesture(node, () => {});

    gesture.arm();
    gesture.disarm();
    expect(node.removed).toEqual(['wheel']);

    gesture.disarm();
    expect(node.removed).toHaveLength(1);
  });

  it('can be armed again after disarming', () => {
    const node = target();
    const gesture = blockingWheelGesture(node, () => {});

    gesture.arm();
    gesture.disarm();
    gesture.arm();

    expect(node.added).toHaveLength(2);
    expect(node.removed).toHaveLength(1);
  });
});
