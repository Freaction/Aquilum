import { syntaxTree } from '@codemirror/language';
import type { EditorState } from '@codemirror/state';
import { findAncestor } from '../syntaxAncestor';

interface SuggestContext {
  from: number;
  to: number;
  query: string;
  prefix: string;
  suffix: string;
}

const WORD = /[\p{L}\p{N}_-]+$/u;
const OPEN = '[[';
const CLOSE = ']]';

export function suggestContext(state: EditorState, position: number): SuggestContext | null {
  if (insideCode(state, position)) return null;
  return bracketContext(state, position) ?? wordContext(state, position);
}

function bracketContext(state: EditorState, position: number): SuggestContext | null {
  const line = state.doc.lineAt(position);
  const head = position - line.from;
  const before = line.text.slice(0, head);
  const open = before.lastIndexOf(OPEN);
  if (open < 0) return null;

  const query = before.slice(open + OPEN.length);
  if (query.includes(CLOSE) || query.includes('|') || query.includes('#')) return null;

  const after = line.text.slice(head);
  const close = after.indexOf(CLOSE);
  const stop = close < 0 ? head : head + close;
  return {
    from: line.from + open + OPEN.length,
    to: line.from + stop,
    query,
    prefix: '',
    suffix: close < 0 ? CLOSE : '',
  };
}

function wordContext(state: EditorState, position: number): SuggestContext | null {
  const line = state.doc.lineAt(position);
  const head = position - line.from;
  const before = line.text.slice(0, head);
  const match = WORD.exec(before);
  if (!match) return null;
  if (before[match.index - 1] === '#') return null;
  return {
    from: line.from + match.index,
    to: position,
    query: match[0],
    prefix: OPEN,
    suffix: CLOSE,
  };
}

function insideCode(state: EditorState, position: number): boolean {
  return findAncestor(
    syntaxTree(state).resolveInner(position, -1),
    (node) => node.name.includes('Code'),
  ) !== null;
}
