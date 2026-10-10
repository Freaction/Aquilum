import { isBookCalloutHeader } from '../blockquoteScan';

export type BookCalloutModel = {
  title: string;
  wikiTarget: string | null;
  author: string;
  cover: string;
  filePath: string;
};

const FIELD_RE = /^(Автор|Обложка|file_path|book_file)\s*:\s*(.*)$/i;
const WIKI_RE = /^\[\[([^\]]+)\]\]$/;

function parseBookCalloutHeader(headerLine: string): {
  title: string;
  wikiTarget: string | null;
} {
  const raw = headerLine.replace(/^>\s*\[!book\]\s*/i, '').trim();
  const wiki = raw.match(WIKI_RE);
  if (wiki) {
    const target = wiki[1].trim();
    return { title: target, wikiTarget: target };
  }
  return { title: raw || 'Книга', wikiTarget: null };
}

export function parseBookCalloutBlock(text: string): BookCalloutModel | null {
  const lines = text.split('\n');
  if (lines.length === 0) return null;
  const first = lines[0].trim();
  if (!isBookCalloutHeader(first)) return null;

  const { title, wikiTarget } = parseBookCalloutHeader(first);
  const model: BookCalloutModel = {
    title,
    wikiTarget,
    author: '',
    cover: '',
    filePath: '',
  };

  for (let i = 1; i < lines.length; i += 1) {
    const body = lines[i].replace(/^>\s?/, '').trim();
    if (!body) continue;
    const match = body.match(FIELD_RE);
    if (!match) continue;
    const key = match[1].toLowerCase();
    const value = match[2].trim();
    if (key === 'автор') model.author = value;
    else if (key === 'обложка') model.cover = value;
    else if (key === 'file_path' || key === 'book_file') model.filePath = value;
  }

  return model;
}
