import type { GraphSnapshot } from '../../modules/graph';

export interface Adjacency {
  offsets: Uint32Array;
  targets: Uint32Array;
}

export function buildAdjacency(snapshot: GraphSnapshot): Adjacency {
  const offsets = new Uint32Array(snapshot.nodeCount + 1);
  for (let slot = 0; slot < snapshot.edges.length; slot += 1) {
    offsets[snapshot.edges[slot] + 1] += 1;
  }
  for (let node = 0; node < snapshot.nodeCount; node += 1) {
    offsets[node + 1] += offsets[node];
  }
  const cursor = offsets.slice();
  const targets = new Uint32Array(snapshot.edges.length);
  for (let edge = 0; edge < snapshot.edgeCount; edge += 1) {
    const left = snapshot.edges[edge * 2];
    const right = snapshot.edges[edge * 2 + 1];
    targets[cursor[left]] = right;
    cursor[left] += 1;
    targets[cursor[right]] = left;
    cursor[right] += 1;
  }
  return { offsets, targets };
}

export function neighbours(adjacency: Adjacency, node: number): Uint32Array {
  return adjacency.targets.subarray(adjacency.offsets[node], adjacency.offsets[node + 1]);
}

export function visitLevels(
  adjacency: Adjacency,
  start: number,
  depth: number,
  limit: number,
  visit: (node: number, level: number) => void,
): void {
  const seen = new Set([start]);
  let frontier = [start];
  visit(start, 0);
  for (let level = 1; level <= depth && frontier.length > 0 && seen.size < limit; level += 1) {
    const next: number[] = [];
    for (const node of frontier) {
      for (const neighbour of neighbours(adjacency, node)) {
        if (seen.has(neighbour)) continue;
        seen.add(neighbour);
        next.push(neighbour);
        visit(neighbour, level);
        if (seen.size >= limit) break;
      }
      if (seen.size >= limit) break;
    }
    frontier = next;
  }
}
