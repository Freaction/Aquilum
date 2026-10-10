import { describe, expect, it } from 'vitest';
import {
  COVER_PATTERN_IDS,
  coverPatternIdFrom,
  coverPatternValue,
  DEFAULT_COVER_PATTERN,
  nextRandomCoverPattern,
} from './coverPatterns';

const css = Object.values(import.meta.glob<string>(
  '../../components/Decorations/coverPatterns/*.css',
  { query: '?raw', import: 'default', eager: true },
)).join('\n');

describe('cover patterns', () => {
  it('every pattern id has a css rule and no rule is orphaned', () => {
    const inCss = [...css.matchAll(/\.q-cover-pattern--([a-z0-9-]+)/g)].map((match) => match[1]);
    expect([...new Set(inCss)].sort()).toEqual([...COVER_PATTERN_IDS].sort());
  });

  it('treats an empty cover value as the default pattern', () => {
    expect(coverPatternIdFrom(undefined)).toBe(DEFAULT_COVER_PATTERN);
    expect(coverPatternIdFrom('')).toBe(DEFAULT_COVER_PATTERN);
  });

  it('resolves known and unknown pattern values', () => {
    expect(coverPatternIdFrom(coverPatternValue('parquet'))).toBe('parquet');
    expect(coverPatternIdFrom('pattern:nope')).toBe(DEFAULT_COVER_PATTERN);
  });

  it('maps legacy builtin ids to a stable pattern', () => {
    expect(coverPatternIdFrom('builtin:Cover-11')).toBe(coverPatternIdFrom('builtin:Cover-11'));
    expect(COVER_PATTERN_IDS).toContain(coverPatternIdFrom('builtin:Cover-11'));
  });

  it('keeps vault paths as images', () => {
    expect(coverPatternIdFrom('Files/my-cover.webp')).toBeNull();
    expect(coverPatternIdFrom('C:/photos/cover.png')).toBeNull();
  });

  it('never repeats the current pattern', () => {
    for (let index = 0; index < 50; index += 1) {
      expect(nextRandomCoverPattern('pattern:denim')).not.toBe('pattern:denim');
    }
  });
});
