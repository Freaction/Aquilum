import type { Text } from '@codemirror/state';
import { isReaderQuoteHeader } from '../../../../modules/docs/bookQuotes';
import {
  isBookCalloutHeader,
  isQuoteBlockContinue,
  trimTrailingEmptyQuoteLines,
} from '../blockquoteScan';

type BookCalloutSpan = {
  from: number;
  to: number;
  text: string;
};

export type BookCalloutGroup = {
  from: number;
  to: number;
  items: BookCalloutSpan[];
};

export function findBookCallouts(doc: Text): BookCalloutSpan[] {
  const spans: BookCalloutSpan[] = [];
  let lineNo = 1;
  while (lineNo <= doc.lines) {
    const line = doc.line(lineNo);
    if (!isBookCalloutHeader(line.text)) {
      lineNo += 1;
      continue;
    }
    const from = line.from;
    let endLine = lineNo;
    while (endLine < doc.lines) {
      const next = doc.line(endLine + 1);
      if (!isQuoteBlockContinue(next.text)) break;
      if (isBookCalloutHeader(next.text)) break;
      if (isReaderQuoteHeader(next.text)) break;
      if (next.text.trim() === '' && !/^>/.test(next.text)) break;
      endLine += 1;
    }
    endLine = trimTrailingEmptyQuoteLines(doc, lineNo, endLine);
    const last = doc.line(endLine);
    spans.push({
      from,
      to: last.to,
      text: doc.sliceString(from, last.to),
    });
    lineNo = endLine + 1;
  }
  return spans;
}

function gapIsBlank(text: string): boolean {
  return /^[\s>]*$/.test(text);
}

export function groupAdjacentBookCallouts(doc: Text): BookCalloutGroup[] {
  const groups: BookCalloutGroup[] = [];
  for (const span of findBookCallouts(doc)) {
    const previous = groups[groups.length - 1];
    if (previous && gapIsBlank(doc.sliceString(previous.to, span.from))) {
      previous.to = span.to;
      previous.items.push(span);
    } else {
      groups.push({ from: span.from, to: span.to, items: [span] });
    }
  }
  return groups;
}
