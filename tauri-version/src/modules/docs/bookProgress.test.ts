import { describe, expect, it } from 'vitest';
import {
  currentFromFraction,
  formatPages,
  formatSyntheticPages,
  normalizeToSyntheticTotal,
  pagesFromByteLength,
  parsePages,
} from './bookProgress';

describe('bookProgress', () => {
  it('derives stable page totals from byte length', () => {
    expect(pagesFromByteLength(1024)).toBe(1);
    expect(pagesFromByteLength(1025)).toBe(2);
  });

  it('normalizes any scale onto synthetic total', () => {
    expect(normalizeToSyntheticTotal('100/500', 1000)).toEqual({ current: 200, total: 1000 });
    expect(formatSyntheticPages('100/500', 1000 * 1024).formatted).toBe('200/1000');
  });

  it('rounds fraction to current page', () => {
    expect(currentFromFraction(0.2, 1000)).toBe(200);
    expect(formatPages(42, 100)).toBe('42/100');
    expect(parsePages(' 42 / 100 ')).toEqual({ current: 42, total: 100 });
  });

  it('guards non-finite page math', () => {
    expect(currentFromFraction(Number.NaN, 1000)).toBe(0);
    expect(formatPages(Number.NaN, 1999)).toBe('0/1999');
  });
});
