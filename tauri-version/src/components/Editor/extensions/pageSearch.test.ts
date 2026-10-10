import { describe, expect, it } from 'vitest';
import { normalizePageSearchIndex } from './pageSearch';

describe('normalizePageSearchIndex', () => {
  it('wraps navigation in both directions', () => {
    expect(normalizePageSearchIndex(3, 3)).toBe(0);
    expect(normalizePageSearchIndex(-1, 3)).toBe(2);
  });

  it('returns zero when there are no matches', () => {
    expect(normalizePageSearchIndex(10, 0)).toBe(0);
  });
});
