import { clamp } from '../../modules/math';
import type { GraphSnapshot } from '../../modules/graph';

export const MIN_LAYOUT_SPACING = 0.6;
export const MIN_SPREAD = 0.6;
export const NODE_GAP_SHARE = 0.49;
export const NODE_TEXTURE_WIDTH = 2048;
export const BASE_RADIUS = 0.055;
export const DEGREE_RADIUS = 0.03;
export const MAX_WORLD_RADIUS = 0.7;
export const SUB_PIXEL_RADIUS = 0.75;
export const MAX_NODE_PIXELS = 160;
export const PICK_SLACK_PIXELS = 4;
export const EDGE_WORLD_HALF_WIDTH = 0.004;
export const EDGE_MIN_HALF_PIXELS = 0.35;
export const EDGE_MAX_HALF_PIXELS = 0.9;
export const MAX_EDGES_PER_FRAME = 600_000;
const HIGHLIGHT_FALLOFF = 0.8;
export const HIGHLIGHT_LEVEL_STEP = 32;
export const HIGHLIGHT_MAX_INTENSITY = 31;
export const DIMMED_STRENGTH = 0.28;

export function highlightState(mark: number): number {
  return mark < 1 ? 0 : HIGHLIGHT_FALLOFF ** (mark - 1);
}

export function dimmedBy(state: number, dimming: number): number {
  return 1 + dimming * ((DIMMED_STRENGTH + (1 - DIMMED_STRENGTH) * state) - 1);
}

function nodeCeiling(spread: number): number {
  return Math.min(MAX_WORLD_RADIUS, MIN_LAYOUT_SPACING * spread * NODE_GAP_SHARE);
}

export function nodeWorldRadius(degree: number, sizeScale = 1, spread = 1): number {
  return Math.min(
    (BASE_RADIUS + DEGREE_RADIUS * Math.sqrt(Math.max(degree, 1))) * sizeScale,
    nodeCeiling(spread),
  );
}

export function nodeRadiusPixels(
  degree: number,
  scale: number,
  sizeScale = 1,
  spread = 1,
): number {
  return clamp(
    nodeWorldRadius(degree, sizeScale, spread) * scale,
    SUB_PIXEL_RADIUS,
    MAX_NODE_PIXELS,
  );
}

export function nodeTextureRows(nodeCount: number): number {
  return Math.max(1, Math.ceil(nodeCount / NODE_TEXTURE_WIDTH));
}

export function nodeTexels(snapshot: GraphSnapshot, rows: number): Float32Array {
  const texels = new Float32Array(NODE_TEXTURE_WIDTH * rows * 4);
  for (let node = 0; node < snapshot.nodeCount; node += 1) {
    const slot = node * 4;
    texels[slot] = snapshot.positions[node * 2];
    texels[slot + 1] = snapshot.positions[node * 2 + 1];
    texels[slot + 2] = snapshot.degrees[node];
    texels[slot + 3] = snapshot.createdDays[node];
  }
  return texels;
}

export function freshnessTexels(snapshot: GraphSnapshot, rows: number): Float32Array {
  const texels = new Float32Array(NODE_TEXTURE_WIDTH * rows);
  for (let node = 0; node < snapshot.nodeCount; node += 1) {
    texels[node] = snapshot.modifiedDays[node];
  }
  return texels;
}
