import type { Text } from '@codemirror/state';
import {
  isReaderQuoteHeader,
  parseReaderQuoteBlock,
  parseReaderQuoteLine,
} from '../../../../modules/docs/bookQuotes';
import {
  isBookCalloutHeader,
  isQuoteBlockContinue,
  trimTrailingEmptyQuoteLines,
} from '../blockquoteScan';

type ReaderQuoteSpan = {
  from: number;
  to: number;
  text: string;
  quoteText: string;
  href: string;
  refLabel?: string;
};

function spanFromLines(
  doc: Text,
  fromLine: number,
  endLine: number,
  text: string,
): ReaderQuoteSpan | null {
  const parsed = parseReaderQuoteBlock(text);
  if (!parsed) return null;
  const from = doc.line(fromLine).from;
  const to = doc.line(endLine).to;
  return { from, to, text, ...parsed };
}

export function findReaderQuotes(doc: Text): ReaderQuoteSpan[] {
  const spans: ReaderQuoteSpan[] = [];
  let lineNo = 1;

  while (lineNo <= doc.lines) {
    const line = doc.line(lineNo);

    if (isReaderQuoteHeader(line.text)) {
      const fromLine = lineNo;
      let endLine = lineNo;
      while (endLine < doc.lines) {
        const next = doc.line(endLine + 1);
        if (!isQuoteBlockContinue(next.text)) break;
        if (isReaderQuoteHeader(next.text) || isBookCalloutHeader(next.text)) break;
        if (next.text.trim() === '' && !/^>/.test(next.text)) break;
        endLine += 1;
      }
      endLine = trimTrailingEmptyQuoteLines(doc, fromLine, endLine);
      const text = doc.sliceString(doc.line(fromLine).from, doc.line(endLine).to);
      const span = spanFromLines(doc, fromLine, endLine, text);
      if (span) spans.push(span);
      lineNo = endLine + 1;
      continue;
    }

    const legacy = parseReaderQuoteLine(line.text);
    if (legacy) {
      spans.push({
        from: line.from,
        to: line.to,
        text: line.text,
        quoteText: legacy.quoteText,
        href: legacy.href,
      });
    }
    lineNo += 1;
  }

  return spans;
}

export function readerQuoteTextRange(span: ReaderQuoteSpan): { from: number; to: number } {
  const header = span.text.match(/^>\s*(?:\[!quote\]\s*)?/)?.[0]?.length ?? 0;
  const marker = span.refLabel ? `[${span.refLabel}](` : '[[';
  const at = span.text.lastIndexOf(marker);
  const body = at > header ? span.text.slice(0, at).trimEnd() : span.text;
  return { from: span.from + header, to: span.from + body.length };
}

export function readerQuoteEditEntryPos(span: ReaderQuoteSpan): number {
  if (!span.refLabel) return span.to;
  const text = readerQuoteTextRange(span);
  return text.to > span.from ? text.to - 1 : span.to;
}
