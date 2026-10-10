import type { EditorState } from '@codemirror/state';
import { syntaxTree } from '@codemirror/language';
import { codeBlocks } from '../codeBlock/blocks';
import { DATAVIEW_LANGUAGE } from './constants';

interface DataviewBlock {
  from: number;
  to: number;
  query: string;
}

export function findDataviewBlocks(state: EditorState): DataviewBlock[] {
  const tree = syntaxTree(state);
  const blocks = codeBlocks(tree, state.doc, [{ from: 0, to: state.doc.length }]);
  const found: DataviewBlock[] = [];

  for (const block of blocks) {
    if (block.info.toLowerCase() !== DATAVIEW_LANGUAGE) continue;
    found.push({ from: block.from, to: block.end, query: queryText(state, block.from, block.end) });
  }
  return found;
}

export function dataviewQueriesInText(text: string): string[] {
  const found: string[] = [];
  let fence: string | null = null;
  let collecting = false;
  let body: string[] = [];

  const finish = () => {
    if (collecting) {
      const query = body.join('\n').trim();
      if (query) found.push(query);
    }
    fence = null;
    collecting = false;
    body = [];
  };

  for (const line of text.split('\n')) {
    const mark = /^\s{0,3}(`{3,}|~{3,})\s*(\S*)/.exec(line);

    if (fence === null) {
      if (!mark) continue;
      fence = mark[1];
      collecting = mark[2].toLowerCase() === DATAVIEW_LANGUAGE;
      body = [];
      continue;
    }

    const closes = mark !== null
      && mark[1][0] === fence[0]
      && mark[1].length >= fence.length
      && mark[2] === '';
    if (closes) {
      finish();
      continue;
    }
    if (collecting) body.push(line);
  }

  finish();
  return found;
}

function queryText(state: EditorState, from: number, to: number): string {
  const first = state.doc.lineAt(from).number;
  const last = state.doc.lineAt(to).number;
  const lines: string[] = [];
  for (let line = first + 1; line <= last; line += 1) {
    const text = state.doc.line(line).text;
    if (line === last && /^\s*(```|~~~)/.test(text)) break;
    lines.push(text);
  }
  return lines.join('\n').trim();
}
