import { GFM, parser } from '@lezer/markdown';
import { editorMarkdownExtensions } from '../markdownConfig';

export const tableCellParser = parser.configure([GFM, ...editorMarkdownExtensions]);

export const SEPARATOR_CELL = /^\s*:?-+:?\s*$/;

export function splitTableRow(line: string): string[] | null {
  const trimmed = line.trim();
  if (!trimmed.includes('|')) return null;
  const body = trimmed.replace(/^\|/, '');
  const cells: string[] = [];
  let start = 0;
  let escaped = false;
  for (let index = 0; index < body.length; index++) {
    if (body[index] === '|' && !escaped) {
      cells.push(body.slice(start, index).trim());
      start = index + 1;
    }
    escaped = body[index] === '\\' && !escaped;
  }
  if (start < body.length || cells.length === 0) cells.push(body.slice(start).trim());
  return cells;
}

export function isSeparatorRow(line: string): boolean {
  const cells = splitTableRow(line);
  if (!cells?.length) return false;
  return cells.every((cell) => SEPARATOR_CELL.test(cell) && cell.replace(/\s/g, '').length > 0);
}
