import type { DataviewCell, DataviewCellPart, DataviewOutput } from '../../../../modules/docs/dataview';
import { samePath } from '../../../../modules/paths';
import { MAX_CACHED_NOTES, UNKNOWN_HEIGHT } from './constants';

export type DataviewResult =
  | { status: 'ready'; output: DataviewOutput }
  | { status: 'failed'; message: string };

type QueryState = {
  result: DataviewResult | null;
  signature: string;
  height: number;
};

const notes = new Map<string, Map<string, QueryState>>();

function noteStates(notePath: string): Map<string, QueryState> {
  const known = notes.get(notePath);
  if (known) {
    notes.delete(notePath);
    notes.set(notePath, known);
    return known;
  }
  const created = new Map<string, QueryState>();
  notes.set(notePath, created);
  for (const oldest of notes.keys()) {
    if (notes.size <= MAX_CACHED_NOTES) break;
    notes.delete(oldest);
  }
  return created;
}

function stateOf(notePath: string, query: string): QueryState | undefined {
  return notes.get(notePath)?.get(query);
}

function writableState(notePath: string, query: string): QueryState {
  const states = noteStates(notePath);
  const known = states.get(query);
  if (known) return known;
  const created: QueryState = { result: null, signature: '', height: UNKNOWN_HEIGHT };
  states.set(query, created);
  return created;
}

export function cachedDataviewResult(notePath: string, query: string): DataviewResult | undefined {
  return stateOf(notePath, query)?.result ?? undefined;
}

export function dataviewResultSignature(notePath: string, query: string): string {
  return stateOf(notePath, query)?.signature ?? '';
}

export function dataviewBlockHeight(notePath: string, query: string): number {
  return stateOf(notePath, query)?.height ?? UNKNOWN_HEIGHT;
}

export function rememberDataviewBlockHeight(notePath: string, query: string, height: number): void {
  if (height <= 0) return;
  const state = stateOf(notePath, query);
  if (!state || state.height === height) return;
  state.height = height;
}

export function rememberDataviewResult(
  notePath: string,
  query: string,
  result: DataviewResult,
): boolean {
  const state = writableState(notePath, query);
  const next = signature(result);
  if (state.result && state.signature === next) return false;
  state.result = result;
  state.signature = next;
  return true;
}

export function forgetDataviewQueriesOutside(notePath: string, queries: Set<string>): void {
  const states = noteStates(notePath);
  for (const query of states.keys()) {
    if (!queries.has(query)) states.delete(query);
  }
}

function shouldExcludeTask(query: string, nextDone: boolean): boolean {
  const whereMatch = query.match(/\bwhere\b([\s\S]*?)(?:\bsort\b|\bgroup\b|\blimit\b|\bflatten\b|$)/i);
  if (!whereMatch) return false;
  const whereClause = whereMatch[1] ?? '';

  const requiresUncompleted = /!\s*(?:task\.)?completed\b/i.test(whereClause)
    || /\b(?:task\.)?completed\s*={1,3}\s*false\b/i.test(whereClause)
    || /\b(?:task\.)?completed\s*!={1,2}\s*true\b/i.test(whereClause);
  if (requiresUncompleted) {
    return nextDone;
  }

  const requiresCompleted = /(?<![!\w])(?:task\.)?completed\s*={1,3}\s*true\b/i.test(whereClause)
    || /(?<![!\w])(?:task\.)?completed\s*!={1,2}\s*false\b/i.test(whereClause)
    || /(?<![!\w])(?:task\.)?completed\b(?!\s*={1,3}\s*false|\s*!={1,2}\s*true)/i.test(whereClause);
  if (requiresCompleted) {
    return !nextDone;
  }

  return false;
}

export function toggleCachedDataviewTask(target: string, line: number): boolean {
  let changed = false;
  for (const states of notes.values()) {
    for (const [query, state] of states.entries()) {
      if (state.result?.status !== 'ready') continue;
      const output = state.result.output;
      let queryModified = false;
      const remainingRows: DataviewCell[] = [];

      for (const row of output.rows) {
        let keepRow = true;
        for (const part of row.parts) {
          if (part.kind === 'check' && samePath(part.target, target) && part.line === line) {
            const nextDone = !part.done;
            if (shouldExcludeTask(query, nextDone)) {
              keepRow = false;
              queryModified = true;
            } else {
              part.done = nextDone;
              queryModified = true;
            }
          }
        }
        if (keepRow) {
          remainingRows.push(row);
        }
      }

      if (queryModified) {
        const dropped = output.rows.length - remainingRows.length;
        output.rows = remainingRows;
        output.total = Math.max(0, output.total - dropped);
        output.truncated = output.total > output.rows.length;
        state.signature = signature(state.result);
        changed = true;
      }
    }
  }
  return changed;
}

function partSignature(part: DataviewCellPart): string {
  if (part.kind === 'progress') return `progress:${part.percent.toFixed(1)}`;
  if (part.kind === 'check') return `check:${part.done}:${part.target}:${part.line}`;
  return `${part.kind}:${part.text}`;
}

function signature(result: DataviewResult): string {
  if (result.status === 'failed') return `failed\0${result.message}`;
  const { output } = result;
  const cells = output.rows
    .map((row) => row.parts.map(partSignature).join(''))
    .join('');
  return [
    'ready',
    output.shape,
    output.title ?? '',
    output.columns.join(''),
    output.total,
    output.truncated,
    cells,
  ].join('\0');
}
