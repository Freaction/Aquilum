import { describe, expect, it } from 'vitest';
import { buildPickGrid, pickNode, visitArea } from './pickGrid';
import { readableAt } from './labelLayer';
import type { GraphSnapshot } from '../../modules/graph';

function snapshot(positions: number[]): GraphSnapshot {
  const nodeCount = positions.length / 2;
  return {
    nodeCount,
    edgeCount: 0,
    epoch: { low: 0, high: 0 },
    positions: new Float32Array(positions),
    createdDays: new Float32Array(nodeCount),
    modifiedDays: new Float32Array(nodeCount),
    degrees: new Uint32Array(nodeCount),
    edges: new Uint32Array(0),
  };
}

describe('pickNode', () => {
  const notes = snapshot([0, 0, 4, 0, -4, 3]);
  const grid = buildPickGrid(notes)!;

  it('catches the note under the pointer', () => {
    expect(pickNode(grid, notes, 4, 0, 60)).toBe(1);
  });

  it('catches nothing in an empty patch', () => {
    expect(pickNode(grid, notes, 2, 2, 60)).toBe(-1);
  });

  it('follows the notes when they are spread apart', () => {
    expect(pickNode(grid, notes, 8, 0, 60, 1, 2)).toBe(1);
    expect(pickNode(grid, notes, 4, 0, 60, 1, 2)).toBe(-1);
  });

  it('keeps the same reach in pixels whatever the spread is', () => {
    const tight = pickNode(grid, notes, 4.05, 0, 60, 1, 1);
    const loose = pickNode(grid, notes, 8.05, 0, 60, 1, 2);

    expect(tight).toBe(1);
    expect(loose).toBe(1);
  });
});

describe('visitArea', () => {
  it('skips the cells the area does not touch', () => {
    const spread: number[] = [];
    for (let node = 0; node < 100; node += 1) {
      spread.push(node % 10, Math.floor(node / 10));
    }
    const notes = snapshot(spread);
    const grid = buildPickGrid(notes)!;
    const seen: number[] = [];

    visitArea(grid, -0.5, -0.5, 0.5, 0.5, (node) => seen.push(node));

    expect(seen).toContain(0);
    expect(seen).not.toContain(99);
    expect(seen.length).toBeLessThan(notes.nodeCount);
  });
});

describe('readableAt', () => {
  it('hides labels while a link is shorter than fifty pixels', () => {
    expect(readableAt(20)).toBe(0);
    expect(readableAt(50)).toBe(0);
  });

  it('fades them in over the next fifty pixels', () => {
    expect(readableAt(75)).toBeCloseTo(0.5, 6);
  });

  it('shows them fully once a link is a hundred pixels long', () => {
    expect(readableAt(100)).toBe(1);
    expect(readableAt(400)).toBe(1);
  });
});
