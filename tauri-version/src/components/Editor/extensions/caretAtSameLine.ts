import { clamp } from '../../../modules/math';

function lineStart(text: string, line: number): number {
  let start = 0;
  for (let index = 0; index < line; index += 1) {
    const next = text.indexOf('\n', start);
    if (next === -1) return -1;
    start = next + 1;
  }
  return start;
}

function lineEnd(text: string, start: number): number {
  const next = text.indexOf('\n', start);
  return next === -1 ? text.length : next;
}

export function caretAtSameLine(previous: string, next: string, position: number): number {
  const clamped = clamp(position, 0, previous.length);
  const line = previous.slice(0, clamped).split('\n').length - 1;
  const column = clamped - lineStart(previous, line);

  const start = lineStart(next, line);
  if (start === -1) return next.length;
  return Math.min(start + column, lineEnd(next, start));
}
