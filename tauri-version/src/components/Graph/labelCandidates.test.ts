import { describe, expect, it } from 'vitest';
import { CandidatePicker, LABEL_LIMIT, type LabelScene } from './labelCandidates';
import { nodeRadiusPixels } from './nodeMetrics';
import { buildPickGrid } from './pickGrid';
import type { NodeReader } from './snapshotStore';
import type { GraphSnapshot } from '../../modules/graph';

function grid(positions: number[]) {
  const nodeCount = positions.length / 2;
  const snapshot: GraphSnapshot = {
    nodeCount,
    edgeCount: 0,
    epoch: { low: 0, high: 0 },
    positions: new Float32Array(positions),
    createdDays: new Float32Array(nodeCount),
    modifiedDays: new Float32Array(nodeCount),
    degrees: new Uint32Array(nodeCount),
    edges: new Uint32Array(0),
  };
  return buildPickGrid(snapshot)!;
}

function reader(
  positions: number[],
  degrees: number[] = [],
  createdDays: number[] = [],
): NodeReader {
  return {
    nodeCount: positions.length / 2,
    x: (node) => positions[node * 2],
    y: (node) => positions[node * 2 + 1],
    degree: (node) => degrees[node] ?? 0,
    createdDay: (node) => createdDays[node] ?? 0,
  };
}

function scene(positions: number[], overrides: Partial<LabelScene> = {}): LabelScene {
  return {
    nodes: reader(positions),
    grid: grid(positions),
    centerX: 0,
    centerY: 0,
    scale: 100,
    sizeScale: 1,
    spread: 1,
    createdFrom: Number.NEGATIVE_INFINITY,
    hovered: -1,
    ...overrides,
  };
}

const WHOLE_VIEW = { width: 100_000, height: 100_000 };

describe('CandidatePicker', () => {
  it('never offers more labels from the viewport than the per-frame limit', () => {
    const positions: number[] = [];
    for (let node = 0; node < LABEL_LIMIT * 3; node += 1) {
      positions.push(node % 30, Math.floor(node / 30));
    }

    const entries = new CandidatePicker().collect(
      scene(positions),
      WHOLE_VIEW,
      new Map(),
      1,
    );

    expect(entries).toHaveLength(LABEL_LIMIT);
  });

  it('keeps exactly the nodes a full sort would keep', () => {
    const positions: number[] = [];
    const degrees: number[] = [];
    const total = LABEL_LIMIT + 60;
    for (let node = 0; node < total; node += 1) {
      positions.push(node % 20, Math.floor(node / 20));
      degrees.push((node * 7919) % 17);
    }
    const nodes = reader(positions, degrees);
    const world = scene(positions, { nodes });

    const entries = new CandidatePicker().collect(world, WHOLE_VIEW, new Map(), 1);
    const kept = entries.map((entry) => entry.node).sort((left, right) => left - right);

    const reference = Array.from({ length: total }, (_, node) => ({
      node,
      radius: nodeRadiusPixels(degrees[node], world.scale, world.sizeScale),
    }));
    reference.sort((left, right) => right.radius - left.radius || left.node - right.node);
    const expected = reference
      .slice(0, LABEL_LIMIT)
      .map((entry) => entry.node)
      .sort((left, right) => left - right);

    expect(kept).toEqual(expected);
  });

  it('carries fading labels that left the viewport but marks them ineligible', () => {
    const positions: number[] = [];
    for (let row = 0; row < 20; row += 1) {
      for (let column = 0; column < 20; column += 1) {
        positions.push(column, row);
      }
    }
    const far = positions.length / 2 - 1;
    const world = scene(positions, { scale: 100 });
    const fades = new Map([[far, 0.6]]);

    const entries = new CandidatePicker().collect(
      world,
      { width: 300, height: 300 },
      fades,
      1,
    );
    const carried = entries.find((entry) => entry.node === far);

    expect(entries.length).toBeLessThan(LABEL_LIMIT);
    expect(carried).toBeDefined();
    expect(carried?.eligible).toBe(false);
    expect(carried?.fade).toBe(0.6);
  });

  it('puts the hovered note first and keeps it eligible', () => {
    const positions: number[] = [];
    const degrees: number[] = [];
    for (let node = 0; node < 50; node += 1) {
      positions.push(node, 0);
      degrees.push(100 - node);
    }
    const world = scene(positions, {
      nodes: reader(positions, degrees),
      hovered: 49,
    });

    const entries = new CandidatePicker().collect(world, WHOLE_VIEW, new Map(), 1);

    expect(entries[0].node).toBe(49);
    expect(entries[0].eligible).toBe(true);
  });

  it('still leads with the hovered note when the viewport overflows the limit', () => {
    const positions: number[] = [];
    const degrees: number[] = [];
    for (let node = 0; node < LABEL_LIMIT * 2; node += 1) {
      positions.push(node % 30, Math.floor(node / 30));
      degrees.push(node === 0 ? 0 : 40);
    }
    const world = scene(positions, {
      nodes: reader(positions, degrees),
      hovered: 0,
    });

    const entries = new CandidatePicker().collect(world, WHOLE_VIEW, new Map(), 1);

    expect(entries[0].node).toBe(0);
    expect(entries).toHaveLength(LABEL_LIMIT + 1);
  });

  it('leaves out notes hidden by the date filter', () => {
    const positions = [0, 0, 1, 0, 2, 0];
    const world = scene(positions, {
      nodes: reader(positions, [], [10, 20, 30]),
      createdFrom: 25,
    });

    const entries = new CandidatePicker().collect(world, WHOLE_VIEW, new Map(), 1);

    expect(entries.map((entry) => entry.node)).toEqual([2]);
  });

  it('collects only the hovered note below the readable threshold', () => {
    const positions = [0, 0, 1, 0, 2, 0];
    const world = scene(positions, { hovered: 1 });

    const entries = new CandidatePicker().collect(world, WHOLE_VIEW, new Map(), 0);

    expect(entries.map((entry) => entry.node)).toEqual([1]);
  });

  it('reuses its candidate objects between frames', () => {
    const positions = [0, 0, 1, 0];
    const world = scene(positions);
    const picker = new CandidatePicker();

    const first = picker.collect(world, WHOLE_VIEW, new Map(), 1);
    const kept = [...first];
    const second = picker.collect(world, WHOLE_VIEW, new Map(), 1);

    expect(second.every((entry) => kept.includes(entry))).toBe(true);
  });
});
