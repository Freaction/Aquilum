export const READER_LINK_SCHEME = 'aquilum-reader:';

type ReaderQuoteHref = {
  cfi: string;
  bookFile?: string;
};

type ParsedReaderQuote = {
  quoteText: string;
  href: string;
  refLabel?: string;
};

const READER_QUOTE_HEADER_RE = /^>\s*\[!quote\]\s*/i;
const READER_QUOTE_REF_BODY_RE = /^(.+?)\s+\[(\d+)\]\((aquilum-reader:[^)]+)\)\s*$/;
const READER_QUOTE_BODY_RE = /^(.+?)\[\[(?:→|↗)\]\]\((aquilum-reader:[^)]+)\)\s*$/;
const READER_QUOTE_LINE_PREFIX_RE = /^>\s/;
const READER_QUOTE_INLINE_BODY_RE = /^\[(.+?)\]\((aquilum-reader:[^)]+)\)$/;

function escapeReaderQuoteText(text: string): string {
  return text.replace(/\\/g, '\\\\').replace(/\[/g, '\\[').replace(/\]/g, '\\]');
}

function unescapeReaderQuoteText(text: string): string {
  return text.replace(/\\([\[\]])/g, '$1');
}

function readerQuoteHref(cfi: string, bookFile?: string): string {
  const params = new URLSearchParams();
  if (bookFile?.trim()) params.set('book', bookFile.trim());
  params.set('cfi', cfi);
  return `${READER_LINK_SCHEME}${params.toString()}`;
}

function parseReaderQuoteBody(body: string): ParsedReaderQuote | null {
  const trimmed = body.trimEnd();
  if (!trimmed) return null;

  const refMatch = trimmed.match(READER_QUOTE_REF_BODY_RE);
  if (refMatch) {
    return {
      quoteText: unescapeReaderQuoteText(refMatch[1].trimEnd()),
      href: refMatch[3],
      refLabel: refMatch[2],
    };
  }

  const legacyMatch = trimmed.match(READER_QUOTE_BODY_RE);
  if (!legacyMatch) return null;
  return {
    quoteText: unescapeReaderQuoteText(legacyMatch[1].trimEnd()),
    href: legacyMatch[2],
  };
}

export function isReaderQuoteHeader(line: string): boolean {
  return READER_QUOTE_HEADER_RE.test(line.trim());
}

export function formatReaderQuoteRef(number: number): string {
  return `[${number}]`;
}

export function formatBookQuote(
  text: string,
  cfi: string,
  bookFile?: string,
  refNumber = 1,
): string {
  const cleaned = text.replace(/\s+/g, ' ').trim();
  if (!cfi) return `> [!quote] ${cleaned}\n`;
  const href = readerQuoteHref(cfi, bookFile);
  const ref = formatReaderQuoteRef(refNumber);
  return `> [!quote] ${escapeReaderQuoteText(cleaned)} ${ref}(${href})\n`;
}

export function parseReaderQuoteBlock(text: string): ParsedReaderQuote | null {
  const lines = text.split('\n');
  if (lines.length === 0) return null;
  const first = lines[0].trimEnd();
  if (isReaderQuoteHeader(first)) {
    return parseReaderQuoteBody(first.replace(READER_QUOTE_HEADER_RE, ''));
  }
  return parseReaderQuoteLine(first);
}

export function parseReaderQuoteLine(line: string): ParsedReaderQuote | null {
  const trimmed = line.trimEnd();
  if (!READER_QUOTE_LINE_PREFIX_RE.test(trimmed)) return null;
  const body = trimmed.replace(READER_QUOTE_LINE_PREFIX_RE, '');

  const parsed = parseReaderQuoteBody(body);
  if (parsed) return parsed;

  const inline = body.match(READER_QUOTE_INLINE_BODY_RE);
  if (!inline) return null;
  return {
    quoteText: unescapeReaderQuoteText(inline[1]),
    href: inline[2],
  };
}

export type BookQuoteRef = {
  cfi: string;
  label: string;
  bookFile?: string;
};

export function collectBookQuoteRefs(docText: string): BookQuoteRef[] {
  const refs: BookQuoteRef[] = [];
  let counter = 0;
  for (const line of docText.split('\n')) {
    const parsed = parseReaderQuoteBlock(line);
    if (!parsed) continue;
    counter += 1;
    const href = parseReaderQuoteHref(parsed.href);
    if (!href) continue;
    refs.push({
      cfi: href.cfi,
      label: parsed.refLabel ?? String(counter),
      bookFile: href.bookFile,
    });
  }
  return refs;
}

export function parseReaderQuoteHref(url: string): ReaderQuoteHref | null {
  const trimmed = url.trim();
  if (!trimmed.startsWith(READER_LINK_SCHEME)) return null;
  const rest = trimmed.slice(READER_LINK_SCHEME.length);
  if (rest.startsWith('cfi=')) {
    try {
      return { cfi: decodeURIComponent(rest.slice(4)) };
    } catch {
      return null;
    }
  }
  try {
    const params = new URLSearchParams(rest);
    const cfi = params.get('cfi');
    if (!cfi) return null;
    const book = params.get('book')?.trim();
    return { cfi, bookFile: book || undefined };
  } catch {
    return null;
  }
}
