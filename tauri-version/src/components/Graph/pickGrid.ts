import { clamp } from '../../modules/math';
import { graphBounds, type GraphSnapshot } from '../../modules/graph';
import { MAX_NODE_PIXELS, nodeRadiusPixels, PICK_SLACK_PIXELS } from './nodeMetrics';

export interface PickGrid {
  cell: number;
  columns: number;
  rows: number;
  minX: number;
  minY: number;
  offsets: Uint32Array;
  nodes: Uint32Array;
}

export function buildPickGrid(snapshot: GraphSnapshot): PickGrid | null {
  if (snapshot.nodeCount === 0) return null;
  const bounds = graphBounds(snapshot);
  const width = Math.max(bounds.maxX - bounds.minX, 1e-3);
  const height = Math.max(bounds.maxY - bounds.minY, 1e-3);
  const target = Math.max(1, Math.floor(Math.sqrt(snapshot.nodeCount / 2)));
  const cell = Math.max(width, height) / target;
  const columns = Math.max(1, Math.ceil(width / cell));
  const rows = Math.max(1, Math.ceil(height / cell));
  const offsets = new Uint32Array(columns * rows + 1);
  const cellOf = new Uint32Array(snapshot.nodeCount);
  for (let node = 0; node < snapshot.nodeCount; node += 1) {
    const column = clamp(
      Math.floor((snapshot.positions[node * 2] - bounds.minX) / cell),
      0,
      columns - 1,
    );
    const row = clamp(
      Math.floor((snapshot.positions[node * 2 + 1] - bounds.minY) / cell),
      0,
      rows - 1,
    );
    const index = row * columns + column;
    cellOf[node] = index;
    offsets[index + 1] += 1;
  }
  for (let index = 0; index < columns * rows; index += 1) {
    offsets[index + 1] += offsets[index];
  }
  const cursor = offsets.slice();
  const nodes = new Uint32Array(snapshot.nodeCount);
  for (let node = 0; node < snapshot.nodeCount; node += 1) {
    nodes[cursor[cellOf[node]]] = node;
    cursor[cellOf[node]] += 1;
  }
  return { cell, columns, rows, minX: bounds.minX, minY: bounds.minY, offsets, nodes };
}

export function visitArea(
  grid: PickGrid,
  minX: number,
  minY: number,
  maxX: number,
  maxY: number,
  visit: (node: number) => void,
): void {
  const minColumn = Math.max(0, Math.floor((minX - grid.minX) / grid.cell));
  const maxColumn = Math.min(grid.columns - 1, Math.floor((maxX - grid.minX) / grid.cell));
  const minRow = Math.max(0, Math.floor((minY - grid.minY) / grid.cell));
  const maxRow = Math.min(grid.rows - 1, Math.floor((maxY - grid.minY) / grid.cell));
  for (let row = minRow; row <= maxRow; row += 1) {
    for (let column = minColumn; column <= maxColumn; column += 1) {
      const cell = row * grid.columns + column;
      for (let slot = grid.offsets[cell]; slot < grid.offsets[cell + 1]; slot += 1) {
        visit(grid.nodes[slot]);
      }
    }
  }
}

export function pickNode(
  grid: PickGrid,
  snapshot: GraphSnapshot,
  worldX: number,
  worldY: number,
  scale: number,
  sizeScale = 1,
  spread = 1,
): number {
  const pixelsPerCell = scale * spread;
  const targetX = worldX / spread;
  const targetY = worldY / spread;
  const reach = (MAX_NODE_PIXELS + PICK_SLACK_PIXELS) / pixelsPerCell;
  let best = -1;
  let bestDistance = Infinity;
  visitArea(grid, targetX - reach, targetY - reach, targetX + reach, targetY + reach, (node) => {
    const dx = snapshot.positions[node * 2] - targetX;
    const dy = snapshot.positions[node * 2 + 1] - targetY;
    const distance = Math.hypot(dx, dy);
    const radius =
      (nodeRadiusPixels(snapshot.degrees[node], scale, sizeScale, spread) + PICK_SLACK_PIXELS)
      / pixelsPerCell;
    if (distance <= radius && distance < bestDistance) {
      bestDistance = distance;
      best = node;
    }
  });
  return best;
}
