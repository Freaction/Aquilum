import type { ColorToken } from '../vaultData';

export function colorForPath(path: string, colors: Record<string, ColorToken>, cascade: boolean): ColorToken | undefined {
  let current = path;
  while (current) {
    if (Object.prototype.hasOwnProperty.call(colors, current)) return colors[current];
    if (!cascade) break;
    current = current.includes('/') ? current.slice(0, current.lastIndexOf('/')) : '';
  }
  return undefined;
}
