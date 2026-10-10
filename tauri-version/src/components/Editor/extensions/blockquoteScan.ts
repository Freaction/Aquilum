import type { Text } from '@codemirror/state';

const CALLOUT_MARKER_RE = /^>\s*(\[![^\]\s]+\])/;

export function isBookCalloutHeader(line: string): boolean {
  return /^>\s*\[!book\]/i.test(line.trim());
}

export function calloutMarkerRange(line: string): { from: number; to: number } | null {
  const match = line.match(CALLOUT_MARKER_RE);
  if (!match) return null;
  return { from: match[0].length - match[1].length, to: match[0].length };
}

export function isQuoteBlockContinue(line: string): boolean {
  return /^>/.test(line) || line.trim() === '';
}

export function trimTrailingEmptyQuoteLines(doc: Text, fromLine: number, endLine: number): number {
  let trimmed = endLine;
  while (trimmed > fromLine) {
    const cur = doc.line(trimmed);
    if (cur.text.replace(/^>\s?/, '').trim() !== '') break;
    trimmed -= 1;
  }
  return trimmed;
}
