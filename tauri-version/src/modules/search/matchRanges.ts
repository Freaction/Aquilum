interface MatchRange {
  start: number;
  end: number;
}

interface MatchOptions {
  wordStartOnly?: boolean;
}

export function findMatchRanges(
  text: string,
  terms: string[],
  { wordStartOnly = true }: MatchOptions = {},
): MatchRange[] {
  if (!text || terms.length === 0) return [];
  const normalized = normalizeWithSpans(text);
  const ranges: MatchRange[] = [];
  for (const term of terms) {
    const normalizedTerm = normalize(term);
    let from = 0;
    while (from < normalized.value.length && normalizedTerm) {
      const start = normalized.value.indexOf(normalizedTerm, from);
      if (start < 0) break;
      if (!wordStartOnly || startsWord(normalized.value, start)) {
        const first = normalized.spans.find((span) => span.normalizedStart === start);
        const normalizedEnd = start + normalizedTerm.length;
        const last = findLastSpan(normalized.spans, normalizedEnd);
        if (first && last) ranges.push({ start: first.originalStart, end: last.originalEnd });
      }
      from = start + 1;
    }
  }
  ranges.sort((left, right) => left.start - right.start || right.end - left.end);
  return ranges.filter((range, index) => index === 0 || range.start >= ranges[index - 1].end);
}

export function flattenMatchRanges(ranges: MatchRange[]): number[] {
  const flat: number[] = [];
  for (const range of ranges) flat.push(range.start, range.end);
  return flat;
}

function startsWord(value: string, start: number): boolean {
  const previous = start > 0 ? value[start - 1] : '';
  return !previous || !/[\p{L}\p{N}]/u.test(previous);
}

interface NormalizedSpan {
  normalizedStart: number;
  normalizedEnd: number;
  originalStart: number;
  originalEnd: number;
}

function normalizeWithSpans(text: string): { value: string; spans: NormalizedSpan[] } {
  let value = '';
  const spans: NormalizedSpan[] = [];
  let originalStart = 0;
  for (const character of text) {
    const originalEnd = originalStart + character.length;
    for (const folded of normalize(character)) {
      const normalizedStart = value.length;
      value += folded;
      spans.push({ normalizedStart, normalizedEnd: value.length, originalStart, originalEnd });
    }
    originalStart = originalEnd;
  }
  return { value, spans };
}

function normalize(value: string): string {
  return value.toLowerCase().normalize('NFD').replace(/\p{M}/gu, '');
}

function findLastSpan(spans: NormalizedSpan[], normalizedEnd: number): NormalizedSpan | undefined {
  for (let index = spans.length - 1; index >= 0; index -= 1) {
    if (spans[index].normalizedEnd === normalizedEnd) return spans[index];
  }
  return undefined;
}
