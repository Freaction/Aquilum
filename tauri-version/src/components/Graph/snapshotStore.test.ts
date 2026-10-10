import { describe, expect, it } from 'vitest';
import { SnapshotStore } from './snapshotStore';
import type { GraphSnapshot } from '../../modules/graph';

function fakeSnapshot(created: number[], modified: number[]): GraphSnapshot {
  const nodeCount = created.length;
  return {
    nodeCount,
    edgeCount: 0,
    epoch: { low: 1, high: 0 },
    positions: new Float32Array(nodeCount * 2),
    createdDays: new Float32Array(created),
    modifiedDays: new Float32Array(modified),
    degrees: new Uint32Array(nodeCount),
    edges: new Uint32Array(0),
  };
}

describe('SnapshotStore date ranges', () => {
  it('computes exact date ranges without artificial day padding', () => {
    const store = new SnapshotStore();
    store.adopt(fakeSnapshot([100.1, 100.2, 100.5], [200.1, 200.8]), false);

    expect(store.created.oldest).toBeCloseTo(100.1);
    expect(store.created.newest).toBeCloseTo(100.5);
    expect(store.freshness.oldest).toBeCloseTo(200.1);
    expect(store.freshness.newest).toBeCloseTo(200.8);
  });

  it('keeps identical dates without adding an artificial day', () => {
    const store = new SnapshotStore();
    store.adopt(fakeSnapshot([50, 50, 50], [60, 60]), false);

    expect(store.created.oldest).toBe(50);
    expect(store.created.newest).toBe(50);
    expect(store.freshness.oldest).toBe(60);
    expect(store.freshness.newest).toBe(60);
  });

  it('skips non-finite dates when computing bounds', () => {
    const store = new SnapshotStore();
    store.adopt(fakeSnapshot([Number.NaN, 10, Number.POSITIVE_INFINITY, 25], [Number.NaN, 30]), false);

    expect(store.created.oldest).toBe(10);
    expect(store.created.newest).toBe(25);
    expect(store.freshness.oldest).toBe(30);
    expect(store.freshness.newest).toBe(30);
  });

  it('falls back to zero bounds when no dates are finite', () => {
    const store = new SnapshotStore();
    store.adopt(fakeSnapshot([Number.NaN], [Number.NaN]), false);

    expect(store.created).toEqual({ oldest: 0, newest: 0 });
    expect(store.freshness).toEqual({ oldest: 0, newest: 0 });
  });
});
