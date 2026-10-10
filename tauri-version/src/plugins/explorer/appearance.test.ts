import { expect, it } from 'vitest';
import { colorForPath } from './appearance';

it('inherits the nearest folder color only with cascade and respects segment boundaries', () => {
  const colors = { A: 'blue', 'A/B': 'red', 'A/B/n.md': 'green' } as const;
  expect(colorForPath('A/B/n.md', colors, true)).toBe('green');
  expect(colorForPath('A/B/other.md', colors, true)).toBe('red');
  expect(colorForPath('A/other.md', colors, true)).toBe('blue');
  expect(colorForPath('A/B/other.md', colors, false)).toBeUndefined();
  expect(colorForPath('AB/other.md', colors, true)).toBeUndefined();
});
