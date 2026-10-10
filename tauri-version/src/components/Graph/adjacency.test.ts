import { describe, expect, it } from 'vitest';
import { buildAdjacency, neighbours, visitLevels } from './adjacency';
import type { GraphSnapshot } from '../../modules/graph';

function snapshot(nodeCount: number, edges: number[]): GraphSnapshot {
  return {
    nodeCount,
    edgeCount: edges.length / 2,
    epoch: { low: 0, high: 0 },
    positions: new Float32Array(nodeCount * 2),
    createdDays: new Float32Array(nodeCount),
    modifiedDays: new Float32Array(nodeCount),
    degrees: new Uint32Array(nodeCount),
    edges: new Uint32Array(edges),
  };
}

describe('buildAdjacency', () => {
  it('lists both ends of every edge', () => {
    const adjacency = buildAdjacency(snapshot(4, [0, 1, 1, 2, 2, 3]));

    expect([...neighbours(adjacency, 0)]).toEqual([1]);
    expect([...neighbours(adjacency, 1)].sort()).toEqual([0, 2]);
    expect([...neighbours(adjacency, 3)]).toEqual([2]);
  });

  it('leaves an isolated note without neighbours', () => {
    const adjacency = buildAdjacency(snapshot(3, [0, 1]));

    expect(neighbours(adjacency, 2)).toHaveLength(0);
  });

  it('handles a graph without edges at all', () => {
    const adjacency = buildAdjacency(snapshot(2, []));

    expect(neighbours(adjacency, 0)).toHaveLength(0);
    expect(neighbours(adjacency, 1)).toHaveLength(0);
  });

  it('keeps every neighbour of a hub', () => {
    const spokes = [0, 1, 0, 2, 0, 3, 0, 4, 0, 5];
    const adjacency = buildAdjacency(snapshot(6, spokes));

    expect([...neighbours(adjacency, 0)].sort()).toEqual([1, 2, 3, 4, 5]);
  });
});

describe('visitLevels', () => {
  const chain = () => buildAdjacency(snapshot(5, [0, 1, 1, 2, 2, 3, 3, 4]));

  function levels(depth: number): Map<number, number> {
    const seen = new Map<number, number>();
    visitLevels(chain(), 0, depth, 1000, (node, level) => seen.set(node, level));
    return seen;
  }

  it('reaches only the neighbours at depth one', () => {
    expect([...levels(1).entries()]).toEqual([[0, 0], [1, 1]]);
  });

  it('walks one step further per depth', () => {
    expect([...levels(2).keys()]).toEqual([0, 1, 2]);
    expect([...levels(3).keys()]).toEqual([0, 1, 2, 3]);
  });

  it('gives every node its shortest distance', () => {
    expect(levels(3).get(3)).toBe(3);
  });

  it('visits a node once even when several paths lead to it', () => {
    const diamond = buildAdjacency(snapshot(4, [0, 1, 0, 2, 1, 3, 2, 3]));
    const visits: number[] = [];

    visitLevels(diamond, 0, 3, 1000, (node) => visits.push(node));

    expect(visits.sort()).toEqual([0, 1, 2, 3]);
  });

  it('stops once the limit is reached', () => {
    const hub = buildAdjacency(snapshot(6, [0, 1, 0, 2, 0, 3, 0, 4, 0, 5]));
    const visits: number[] = [];

    visitLevels(hub, 0, 2, 3, (node) => visits.push(node));

    expect(visits).toHaveLength(3);
  });

  it('lights a lone note without touching anything else', () => {
    const visits: number[] = [];

    visitLevels(buildAdjacency(snapshot(3, [0, 1])), 2, 3, 1000, (node) => visits.push(node));

    expect(visits).toEqual([2]);
  });
});
