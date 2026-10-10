import { describe, expect, it } from 'vitest';
import { buildAdjacency } from './adjacency';
import { HighlightMap } from './highlightMap';
import { HIGHLIGHT_LEVEL_STEP, NODE_TEXTURE_WIDTH, nodeTextureRows } from './nodeMetrics';
import type { GraphSnapshot } from '../../modules/graph';

const FRAME = 1 / 60;

const markOf = (map: HighlightMap, node: number) =>
  Math.floor(map.data[node] / HIGHLIGHT_LEVEL_STEP);

function chain(nodeCount: number, edges: number[]): GraphSnapshot {
  return {
    edges: new Uint32Array(edges),
    edgeCount: edges.length / 2,
    nodeCount,
  } as unknown as GraphSnapshot;
}

function mapOver(nodeCount: number, edges: number[]) {
  const map = new HighlightMap();
  map.resize(nodeTextureRows(nodeCount));
  return { map, adjacency: buildAdjacency(chain(nodeCount, edges)) };
}

function settle(map: HighlightMap): number {
  let frames = 0;
  while (map.advance(FRAME) && frames < 600) frames += 1;
  return frames;
}

describe('HighlightMap', () => {
  it('raises the hovered node over several frames instead of at once', () => {
    const { map, adjacency } = mapOver(3, [0, 1, 1, 2]);

    map.lightUp(adjacency, 1, 1);
    expect(map.stateOf(1)).toBe(0);

    map.advance(FRAME);
    const afterOneFrame = map.stateOf(1);
    expect(afterOneFrame).toBeGreaterThan(0);
    expect(afterOneFrame).toBeLessThan(1);

    settle(map);
    expect(map.stateOf(1)).toBe(1);
  });

  it('dims the rest by the same animation, not by a separate one', () => {
    const { map, adjacency } = mapOver(3, [0, 1, 1, 2]);

    map.lightUp(adjacency, 1, 1);
    expect(map.dimming).toBe(0);
    map.advance(FRAME);
    expect(map.dimming).toBeGreaterThan(0);
    expect(map.dimming).toBeLessThan(1);

    settle(map);
    expect(map.dimming).toBe(1);
  });

  it('lights linked notes weaker than the hovered one', () => {
    const { map, adjacency } = mapOver(3, [0, 1, 1, 2]);

    map.lightUp(adjacency, 1, 1);
    settle(map);

    expect(map.stateOf(1)).toBe(1);
    expect(map.stateOf(0)).toBeGreaterThan(0);
    expect(map.stateOf(0)).toBeLessThan(map.stateOf(1));
    expect(map.stateOf(2)).toBe(map.stateOf(0));
  });

  it('fades back to nothing when the pointer leaves', () => {
    const { map, adjacency } = mapOver(3, [0, 1, 1, 2]);

    map.lightUp(adjacency, 1, 1);
    settle(map);
    map.lightUp(adjacency, -1, 1);

    map.advance(FRAME);
    expect(map.stateOf(1)).toBeLessThan(1);
    expect(map.stateOf(1)).toBeGreaterThan(0);

    settle(map);
    expect(map.stateOf(1)).toBe(0);
    expect(map.stateOf(0)).toBe(0);
    expect(map.dimming).toBe(0);
  });

  it('stops asking for frames once everything settled', () => {
    const { map, adjacency } = mapOver(3, [0, 1, 1, 2]);

    map.lightUp(adjacency, 1, 1);
    const frames = settle(map);

    expect(frames).toBeGreaterThan(1);
    expect(frames).toBeLessThan(120);
    expect(map.advance(FRAME)).toBe(false);
  });

  it('marks only the rows it touched as dirty', () => {
    const { map, adjacency } = mapOver(3, [0, 1, 1, 2]);
    map.settle();
    expect(map.dirtyRows().rowCount).toBe(0);

    map.lightUp(adjacency, 1, 1);
    map.advance(FRAME);
    expect(map.dirtyRows()).toEqual({ firstRow: 0, rowCount: 1 });
    expect(map.data.length).toBe(NODE_TEXTURE_WIDTH);
  });

  it('brings the hovered node to full brightness within a fifth of a second', () => {
    const { map, adjacency } = mapOver(3, [0, 1, 1, 2]);

    map.lightUp(adjacency, 1, 1);
    let elapsed = 0;
    while (map.stateOf(1) < 1 && elapsed < 1) {
      map.advance(FRAME);
      elapsed += FRAME;
    }

    expect(map.stateOf(1)).toBe(1);
    expect(elapsed).toBeLessThan(0.2);
  });

  it('gives two linked neighbours the same level, so the edge between them stays dark', () => {
    const { map, adjacency } = mapOver(3, [0, 1, 1, 2, 0, 2]);

    map.lightUp(adjacency, 1, 1);
    settle(map);

    expect(markOf(map, 1)).toBe(1);
    expect(markOf(map, 0)).toBe(2);
    expect(markOf(map, 2)).toBe(2);
  });

  it('keeps the level in the byte while the light fades out', () => {
    const { map, adjacency } = mapOver(3, [0, 1, 1, 2]);

    map.lightUp(adjacency, 1, 1);
    settle(map);
    map.lightUp(adjacency, -1, 1);
    map.advance(FRAME);

    expect(map.stateOf(0)).toBeGreaterThan(0);
    expect(markOf(map, 0)).toBe(2);
    expect(markOf(map, 1)).toBe(1);

    settle(map);
    expect(markOf(map, 0)).toBe(0);
  });

  it('hands the pointer over between nodes without a jump', () => {
    const { map, adjacency } = mapOver(3, [0, 1, 1, 2]);

    map.lightUp(adjacency, 1, 1);
    settle(map);
    map.lightUp(adjacency, 2, 1);

    expect(map.stateOf(2)).toBeLessThan(1);
    expect(map.stateOf(0)).toBeGreaterThan(0);
    map.advance(FRAME);
    expect(map.stateOf(0)).toBeGreaterThan(0);
    expect(map.stateOf(0)).toBeLessThan(1);

    settle(map);
    expect(map.stateOf(2)).toBe(1);
    expect(map.stateOf(0)).toBe(0);
  });
});
