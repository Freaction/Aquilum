import { clamp } from '../../modules/math';

const EDGE_REFERENCE_COUNT = 20_000;

type Color = [number, number, number, number];

export interface Palette {
  backdrop: Color;
  fill: Color;
  outline: Color;
  edge: Color;
  edgeActive: Color;
  cold: Color;
  hot: Color;
  label: Color;
  labelFocus: Color;
  labelHalo: Color;
}

let probe: CanvasRenderingContext2D | null = null;

export function readPalette(element: HTMLElement): Palette {
  const styles = getComputedStyle(element);
  return {
    backdrop: resolveColor(element, styles.getPropertyValue('--q-graph-backdrop'), [1, 1, 1, 1]),
    fill: resolveColor(element, styles.getPropertyValue('--q-graph-node'), [0.29, 0.45, 0.93, 0.95]),
    outline: resolveColor(element, styles.getPropertyValue('--q-graph-node-rim'), [1, 1, 1, 1]),
    edge: resolveColor(element, styles.getPropertyValue('--q-graph-edge'), [0.5, 0.52, 0.56, 0.5]),
    edgeActive: resolveColor(
      element,
      styles.getPropertyValue('--q-graph-edge-active'),
      [0.95, 0.55, 0.16, 0.9],
    ),
    cold: resolveColor(element, styles.getPropertyValue('--q-graph-heat-cold'), [0.35, 0.42, 0.55, 0.9]),
    hot: resolveColor(element, styles.getPropertyValue('--q-graph-heat-hot'), [0.98, 0.45, 0.2, 1]),
    label: resolveColor(element, styles.getPropertyValue('--q-graph-label'), [0.4, 0.42, 0.46, 1]),
    labelFocus: resolveColor(
      element,
      styles.getPropertyValue('--q-graph-label-focus'),
      [0.1, 0.11, 0.13, 1],
    ),
    labelHalo: resolveColor(element, styles.getPropertyValue('--q-graph-label-halo'), [1, 1, 1, 1]),
  };
}

export function fadeEdges(color: Color, edgeCount: number): Color {
  const density = clamp(Math.sqrt(EDGE_REFERENCE_COUNT / Math.max(edgeCount, 1)), 0.3, 1);
  return [color[0], color[1], color[2], color[3] * density];
}

function parseColorChannels(color: string): Color | null {
  const match = color.match(
    /^rgba?\(\s*([\d.]+)[,\s]+([\d.]+)[,\s]+([\d.]+)(?:[,\s/]+([\d.]+))?\s*\)$/i,
  );
  if (!match) return null;
  return [
    Number(match[1]) / 255,
    Number(match[2]) / 255,
    Number(match[3]) / 255,
    match[4] !== undefined ? Number(match[4]) : 1,
  ];
}

function resolveColor(element: HTMLElement, value: string, fallback: Color): Color {
  const painted = value.trim();
  if (!painted) return fallback;
  let resolved = painted;
  if (painted.includes('var(')) {
    element.style.color = painted;
    resolved = getComputedStyle(element).color;
    element.style.color = '';
  }
  const parsed = parseColorChannels(resolved);
  if (parsed) return parsed;
  const context = colorProbe();
  if (!context) return fallback;
  context.clearRect(0, 0, 1, 1);
  context.fillStyle = '#000000';
  context.fillStyle = resolved;
  if (context.fillStyle === '#000000' && !isBlack(resolved)) return fallback;
  context.fillRect(0, 0, 1, 1);
  const [red, green, blue, alpha] = context.getImageData(0, 0, 1, 1).data;
  return [red / 255, green / 255, blue / 255, alpha / 255];
}

function isBlack(value: string): boolean {
  const lowered = value.toLowerCase();
  return lowered === '#000' || lowered === '#000000' || lowered === 'black';
}

function colorProbe(): CanvasRenderingContext2D | null {
  if (probe) return probe;
  const canvas = document.createElement('canvas');
  canvas.width = 1;
  canvas.height = 1;
  probe = canvas.getContext('2d', { willReadFrequently: true });
  return probe;
}
