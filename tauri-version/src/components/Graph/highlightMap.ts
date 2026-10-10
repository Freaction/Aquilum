import { visitLevels, type Adjacency } from './adjacency';
import { approachStep } from './easing';
import {
  highlightState,
  HIGHLIGHT_LEVEL_STEP,
  HIGHLIGHT_MAX_INTENSITY,
  NODE_TEXTURE_WIDTH,
} from './nodeMetrics';

const HIGHLIGHT_LIMIT = 20_000;
const FADE_SECONDS = 0.055;
const SETTLED_DIMMING = 1 / 255;

function markOf(packed: number): number {
  return Math.floor(packed / HIGHLIGHT_LEVEL_STEP);
}

function intensityOf(packed: number): number {
  return packed % HIGHLIGHT_LEVEL_STEP;
}

function pack(mark: number, intensity: number): number {
  return intensity === 0 ? 0 : mark * HIGHLIGHT_LEVEL_STEP + intensity;
}

export interface DirtyRows {
  firstRow: number;
  rowCount: number;
}

export class HighlightMap {
  data = new Uint8Array(0);
  dimming = 0;
  private targets = new Uint8Array(0);
  private moving = new Set<number>();
  private lit: number[] = [];
  private dimmingTarget = 0;
  private firstDirtyRow = Number.MAX_SAFE_INTEGER;
  private lastDirtyRow = -1;

  resize(rows: number): void {
    this.data = new Uint8Array(NODE_TEXTURE_WIDTH * rows);
    this.targets = new Uint8Array(NODE_TEXTURE_WIDTH * rows);
    this.moving.clear();
    this.lit = [];
    this.dimming = 0;
    this.dimmingTarget = 0;
    this.settle();
  }

  lightUp(adjacency: Adjacency | null, node: number, depth: number): void {
    for (const member of this.lit) this.retarget(member, 0);
    this.lit.length = 0;
    this.dimmingTarget = node >= 0 && adjacency ? 1 : 0;
    if (node < 0 || !adjacency) return;
    visitLevels(adjacency, node, depth, HIGHLIGHT_LIMIT, (member, level) => {
      this.retarget(member, level + 1);
      this.lit.push(member);
    });
  }

  advance(seconds: number): boolean {
    const step = approachStep(seconds, FADE_SECONDS);
    const dimmingGap = this.dimmingTarget - this.dimming;
    this.dimming = Math.abs(dimmingGap) < SETTLED_DIMMING
      ? this.dimmingTarget
      : this.dimming + dimmingGap * step;

    for (const node of this.moving) {
      const target = this.targets[node];
      const current = this.data[node];
      const goal = intensityOf(target);
      const from = intensityOf(current);
      const gap = goal - from;
      const stride = gap * step;
      const intensity = from + (Math.abs(stride) < 1 ? Math.sign(gap) : Math.round(stride));
      const mark = goal === 0 ? markOf(current) : markOf(target);
      this.data[node] = pack(mark, intensity);
      this.markRowDirty(node);
      if (intensity === goal) this.moving.delete(node);
    }

    return this.moving.size > 0 || this.dimming !== this.dimmingTarget;
  }

  stateOf(node: number): number {
    return intensityOf(this.data[node] ?? 0) / HIGHLIGHT_MAX_INTENSITY;
  }

  dirtyRows(): DirtyRows {
    if (this.lastDirtyRow < this.firstDirtyRow) return { firstRow: 0, rowCount: 0 };
    return {
      firstRow: this.firstDirtyRow,
      rowCount: this.lastDirtyRow - this.firstDirtyRow + 1,
    };
  }

  settle(): void {
    this.firstDirtyRow = Number.MAX_SAFE_INTEGER;
    this.lastDirtyRow = -1;
  }

  private retarget(node: number, mark: number): void {
    const intensity = Math.round(highlightState(mark) * HIGHLIGHT_MAX_INTENSITY);
    this.targets[node] = pack(mark, intensity);
    if (intensityOf(this.data[node]) === intensity) this.moving.delete(node);
    else this.moving.add(node);
  }

  private markRowDirty(node: number): void {
    const row = (node / NODE_TEXTURE_WIDTH) | 0;
    if (row < this.firstDirtyRow) this.firstDirtyRow = row;
    if (row > this.lastDirtyRow) this.lastDirtyRow = row;
  }
}
