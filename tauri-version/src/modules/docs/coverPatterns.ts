const COVER_PATTERN_PREFIX = 'pattern:';

const LEGACY_IMAGE_PREFIX = 'builtin:';

export const COVER_PATTERN_IDS = [
  'origami',
  'scales',
  'chevron',
  'arrows',
  'moire',
  'ruby',
  'rings',
  'notebook',
  'dotgrid',
  'parquet',
  'tartan',
  'plaid',
  'pineapple',
  'stripes',
  'zigzag',
  'hearts',
  'cubes',
  'fan',
  'blueprint',
  'neon',
  'lattice',
  'quilt',
  'foliage',
  'ripples',
  'trellis',
  'dreams',
  'polka',
  'weave',
  'quadrants',
  'woven-tiles',
  'city',
  'tunnel',
] as const;

export type CoverPatternId = (typeof COVER_PATTERN_IDS)[number];

export const DEFAULT_COVER_PATTERN: CoverPatternId = COVER_PATTERN_IDS[0];

export function coverPatternClassName(id: CoverPatternId): string {
  return `q-cover-pattern q-cover-pattern--${id}`;
}

export function coverPatternValue(id: CoverPatternId): string {
  return `${COVER_PATTERN_PREFIX}${id}`;
}

function isCoverPatternId(value: string): value is CoverPatternId {
  return (COVER_PATTERN_IDS as readonly string[]).includes(value);
}

function patternForLegacyId(legacyId: string): CoverPatternId {
  let hash = 0;
  for (let index = 0; index < legacyId.length; index += 1) {
    hash = (hash * 31 + legacyId.charCodeAt(index)) % 1000003;
  }
  return COVER_PATTERN_IDS[hash % COVER_PATTERN_IDS.length];
}

export function coverPatternIdFrom(value: string | undefined): CoverPatternId | null {
  const trimmed = value?.trim();
  if (!trimmed) return DEFAULT_COVER_PATTERN;
  if (trimmed.startsWith(COVER_PATTERN_PREFIX)) {
    const id = trimmed.slice(COVER_PATTERN_PREFIX.length);
    return isCoverPatternId(id) ? id : DEFAULT_COVER_PATTERN;
  }
  if (trimmed.startsWith(LEGACY_IMAGE_PREFIX)) {
    return patternForLegacyId(trimmed.slice(LEGACY_IMAGE_PREFIX.length));
  }
  return null;
}

export function nextRandomCoverPattern(current: string | undefined): string {
  const currentId = coverPatternIdFrom(current);
  const pool = COVER_PATTERN_IDS.filter((id) => id !== currentId);
  const list = pool.length ? pool : COVER_PATTERN_IDS;
  return coverPatternValue(list[Math.floor(Math.random() * list.length)]);
}
