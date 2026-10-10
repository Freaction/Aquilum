import { nodeRadiusPixels } from './nodeMetrics';
import { visitArea, type PickGrid } from './pickGrid';
import type { NodeReader } from './snapshotStore';

export const LABEL_LIMIT = 140;

export interface Candidate {
  node: number;
  radius: number;
  fade: number;
  eligible: boolean;
}

export interface LabelScene {
  nodes: NodeReader;
  grid: PickGrid | null;
  centerX: number;
  centerY: number;
  scale: number;
  sizeScale: number;
  spread: number;
  createdFrom: number;
  hovered: number;
}

interface ViewSize {
  width: number;
  height: number;
}

function byRank(left: Candidate, right: Candidate): number {
  return right.fade - left.fade || right.radius - left.radius || left.node - right.node;
}

export class CandidatePicker {
  private readonly pool: Candidate[] = [];
  private readonly order: Candidate[] = [];
  private readonly taken = new Set<number>();
  private limit = LABEL_LIMIT;
  private used = 0;

  setLimit(limit: number): void {
    this.limit = Math.max(1, limit);
  }

  collect(
    scene: LabelScene,
    view: ViewSize,
    fades: Map<number, number>,
    readable: number,
  ): Candidate[] {
    this.used = 0;
    this.order.length = 0;
    if (readable > 0) this.sweepViewport(scene, view, fades);
    this.order.sort(byRank);
    if (this.order.length > this.limit) this.order.length = this.limit;
    if (scene.hovered >= 0) this.order.unshift(this.claim(scene, fades, scene.hovered, true));
    this.taken.clear();
    for (const entry of this.order) this.taken.add(entry.node);
    for (const node of fades.keys()) {
      if (this.taken.has(node)) continue;
      this.order.push(this.claim(scene, fades, node, false));
    }
    return this.order;
  }

  private sweepViewport(
    scene: LabelScene,
    view: ViewSize,
    fades: Map<number, number>,
  ): void {
    const grid = scene.grid;
    if (!grid) return;
    const nodes = scene.nodes;
    const pixelsPerUnit = scene.scale * scene.spread;
    const reachX = view.width / 2 / pixelsPerUnit;
    const reachY = view.height / 2 / pixelsPerUnit;
    const centerX = scene.centerX / scene.spread;
    const centerY = scene.centerY / scene.spread;
    visitArea(
      grid,
      centerX - reachX,
      centerY - reachY,
      centerX + reachX,
      centerY + reachY,
      (node) => {
        if (node === scene.hovered) return;
        if (nodes.createdDay(node) < scene.createdFrom) return;
        this.order.push(this.claim(scene, fades, node, true));
      },
    );
  }

  private claim(
    scene: LabelScene,
    fades: Map<number, number>,
    node: number,
    eligible: boolean,
  ): Candidate {
    const slot = this.pool[this.used]
      ?? (this.pool[this.used] = { node: 0, radius: 0, fade: 0, eligible: false });
    this.used += 1;
    slot.node = node;
    slot.radius = nodeRadiusPixels(
      scene.nodes.degree(node),
      scene.scale,
      scene.sizeScale,
      scene.spread,
    );
    slot.fade = fades.get(node) ?? 0;
    slot.eligible = eligible;
    return slot;
  }
}
