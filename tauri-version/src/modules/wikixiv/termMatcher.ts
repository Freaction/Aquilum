function normalize(value: string): string {
  return value
    .normalize('NFKC')
    .replace(/[\u200B-\u200D\uFEFF]/g, '')
    .replace(/\s+/g, ' ')
    .trim()
    .toLocaleLowerCase();
}

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

export function wikiTermMatcher(terms: readonly string[]): RegExp | null {
  const pattern = terms
    .map(normalize)
    .filter(Boolean)
    .sort((left, right) => right.length - left.length)
    .map(escapeRegExp)
    .join('|');
  return pattern
    ? new RegExp(`(?<![\\p{L}\\p{N}])(?:${pattern})(?![\\p{L}\\p{N}])`, 'giu')
    : null;
}
