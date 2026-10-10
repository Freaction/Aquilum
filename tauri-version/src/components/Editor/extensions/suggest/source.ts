import type { CompletionContext, CompletionResult } from '@codemirror/autocomplete';
import {
  findMatchRanges,
  flattenMatchRanges,
  suggestNotes,
  type NoteSuggestion,
} from '../../../../modules/search';
import { applyNoteLink } from './apply';
import { suggestContext } from './context';

interface NoteSuggestOptions {
  workspacePath: string | null;
  minChars: number;
  limit: number;
}

const EMPTY_MEMORY_MS = 3000;

export function noteCompletionSource(options: NoteSuggestOptions) {
  let withoutMatches: { query: string; at: number } | null = null;

  return async (context: CompletionContext): Promise<CompletionResult | null> => {
    const where = suggestContext(context.state, context.pos);
    if (!where) return null;
    if (!context.explicit && where.query.length < options.minChars) return null;
    if (!options.workspacePath) return null;

    const query = where.query.toLowerCase();
    if (cannotMatch(withoutMatches, query)) return null;

    let notes: NoteSuggestion[];
    try {
      notes = await suggestNotes(options.workspacePath, where.query, options.limit);
    } catch (error) {
      console.error('Не удалось получить подсказки заметок', error);
      return null;
    }
    if (context.aborted) return null;
    if (notes.length === 0) {
      withoutMatches = { query, at: Date.now() };
      return null;
    }
    withoutMatches = null;

    return {
      from: where.from,
      to: where.to,
      filter: false,
      getMatch: (completion) => flattenMatchRanges(
        findMatchRanges(completion.label, [where.query], { wordStartOnly: false }),
      ),
      options: notes.map((note) => ({ label: note.title, apply: applyNoteLink })),
    };
  };
}

function cannotMatch(empty: { query: string; at: number } | null, query: string): boolean {
  if (!empty || Date.now() - empty.at > EMPTY_MEMORY_MS) return false;
  return query.startsWith(empty.query);
}
