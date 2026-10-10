import type { ReactNode } from 'react';
import { findMatchRanges } from '../../modules/search/matchRanges';

export function highlightMatches(
  text: string,
  terms: string[],
  options?: Parameters<typeof findMatchRanges>[2],
): ReactNode {
  const ranges = findMatchRanges(text, terms, options);
  if (ranges.length === 0) return text;

  const nodes: ReactNode[] = [];
  let offset = 0;
  for (const range of ranges) {
    if (range.start > offset) nodes.push(text.slice(offset, range.start));
    nodes.push(<mark key={`${range.start}-${range.end}`}>{text.slice(range.start, range.end)}</mark>);
    offset = range.end;
  }
  if (offset < text.length) nodes.push(text.slice(offset));
  return nodes;
}
