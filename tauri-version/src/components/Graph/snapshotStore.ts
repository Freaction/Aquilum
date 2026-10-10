import { graphBounds, type GraphBounds, type GraphSnapshot } from '../../modules/graph';
import { buildAdjacency, type Adjacency } from './adjacency';
import { freshnessTexels, nodeTexels, nodeTextureRows } from './nodeMetrics';
import { buildPickGrid, type PickGrid } from './pickGrid';

const ANIMATED_NODE_LIMIT = 100_000;
const MORPH_SECONDS = 0.45;

export interface GraphDateRange {
  oldest: number;
  newest: number;
}

interface DisplayPositions {
  x(node: number): number;
  y(node: number): number;
}

export interface NodeReader extends DisplayPositions {
  readonly nodeCount: number;
  degree(node: number): number;
  createdDay(node: number): number;
}

export class SnapshotStore implements NodeReader {
  adjacency: Adjacency | null = null;
  grid: PickGrid | null = null;
  texels = new Float32Array(0);
  rows = 1;
  freshness: GraphDateRange = { oldest: 0, newest: 1 };
  created: GraphDateRange = { oldest: 0, newest: 1 };
  private current: GraphSnapshot | null = null;
  private previousPositions: Float32Array | null = null;
  private transition = 1;

  get snapshot(): GraphSnapshot | null {
    return this.current;
  }

  get nodeCount(): number {
    return this.current?.nodeCount ?? 0;
  }

  get edgeCount(): number {
    return this.current?.edgeCount ?? 0;
  }

  morphsInto(snapshot: GraphSnapshot, keepCamera: boolean): boolean {
    return keepCamera
      && this.current !== null
      && this.current.nodeCount === snapshot.nodeCount
      && snapshot.nodeCount <= ANIMATED_NODE_LIMIT;
  }

  adopt(snapshot: GraphSnapshot, morphs: boolean): void {
    this.previousPositions = morphs ? this.current?.positions.slice() ?? null : null;
    this.transition = morphs ? 0 : 1;
    this.current = snapshot;
    this.adjacency = buildAdjacency(snapshot);
    this.grid = buildPickGrid(snapshot);
    this.freshness = dateRange(snapshot.modifiedDays, snapshot.nodeCount);
    this.created = dateRange(snapshot.createdDays, snapshot.nodeCount);
    this.rows = nodeTextureRows(snapshot.nodeCount);
    this.texels = nodeTexels(snapshot, this.rows);
    if (morphs) this.writePositions(0);
  }

  freshnessTexture(): Float32Array {
    if (!this.current) return new Float32Array(0);
    return freshnessTexels(this.current, this.rows);
  }

  createdRange(): GraphDateRange {
    return this.created;
  }

  createdDay(node: number): number {
    return this.current?.createdDays[node] ?? Number.NEGATIVE_INFINITY;
  }

  degree(node: number): number {
    return this.current?.degrees[node] ?? 0;
  }

  bounds(spread: number): GraphBounds {
    const bounds = graphBounds(this.current!);
    return {
      minX: bounds.minX * spread,
      minY: bounds.minY * spread,
      maxX: bounds.maxX * spread,
      maxY: bounds.maxY * spread,
    };
  }

  isMorphing(): boolean {
    return this.transition < 1;
  }

  advanceMorph(seconds: number): boolean {
    if (this.transition >= 1) return false;
    this.transition = Math.min(1, this.transition + seconds / MORPH_SECONDS);
    this.writePositions(this.transition);
    if (this.transition >= 1) this.previousPositions = null;
    return true;
  }

  x(node: number): number {
    return this.texels[node * 4];
  }

  y(node: number): number {
    return this.texels[node * 4 + 1];
  }

  private writePositions(progress: number): void {
    const snapshot = this.current;
    const from = this.previousPositions;
    if (!snapshot || !from) return;
    const eased = progress * progress * (3 - 2 * progress);
    for (let node = 0; node < snapshot.nodeCount; node += 1) {
      const slot = node * 4;
      const pair = node * 2;
      this.texels[slot] = from[pair] + (snapshot.positions[pair] - from[pair]) * eased;
      this.texels[slot + 1] =
        from[pair + 1] + (snapshot.positions[pair + 1] - from[pair + 1]) * eased;
    }
  }
}

function dateRange(days: Float32Array, count: number): GraphDateRange {
  let oldest = Infinity;
  let newest = -Infinity;
  for (let node = 0; node < count; node += 1) {
    const day = days[node];
    if (!Number.isFinite(day)) continue;
    if (day < oldest) oldest = day;
    if (day > newest) newest = day;
  }
  if (!Number.isFinite(oldest) || !Number.isFinite(newest)) return { oldest: 0, newest: 0 };
  return { oldest, newest };
}
