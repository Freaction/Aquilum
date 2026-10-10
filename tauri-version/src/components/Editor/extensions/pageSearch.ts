import { SearchQuery } from '@codemirror/search';
import { StateEffect, StateField, type EditorState, type Extension } from '@codemirror/state';
import { Decoration, EditorView } from '@codemirror/view';

interface PageSearchState { query: string; activeIndex: number; activeOnly: boolean }
interface MatchRange { from: number; to: number }
export interface PageSearchSnapshot { current: number; total: number; activeOnly: boolean }

const setPageSearch = StateEffect.define<PageSearchState>();

function getMatches(state: EditorState, query: string): MatchRange[] {
  if (!query) return [];
  const cursor = new SearchQuery({ search: query }).getCursor(state);
  const matches: MatchRange[] = [];
  for (let match = cursor.next(); !match.done; match = cursor.next()) matches.push(match.value);
  return matches;
}

const pageSearchField = StateField.define<PageSearchState>({
  create: () => ({ query: '', activeIndex: 0, activeOnly: false }),
  update(value, transaction) {
    for (const effect of transaction.effects) if (effect.is(setPageSearch)) return effect.value;
    if (transaction.docChanged && value.query) {
      const total = getMatches(transaction.state, value.query).length;
      return { ...value, activeIndex: normalizePageSearchIndex(value.activeIndex, total) };
    }
    return value;
  },
  provide: (field) => EditorView.decorations.compute([field, 'doc'], (state) => {
    const search = state.field(field);
    const matches = getMatches(state, search.query);
    return Decoration.set(matches.flatMap((match, index) =>
      search.activeOnly && index !== search.activeIndex
        ? []
        : [Decoration.mark({ class: 'q-page-search-match' }).range(match.from, match.to)],
    ));
  }),
});

export const pageSearchExtension: Extension = pageSearchField;

export function normalizePageSearchIndex(index: number, total: number): number {
  return total ? ((index % total) + total) % total : 0;
}

export function updatePageSearch(
  view: EditorView,
  query: string,
  activeIndex: number,
  activeOnly: boolean,
): PageSearchSnapshot {
  const matches = getMatches(view.state, query);
  const normalizedIndex = normalizePageSearchIndex(activeIndex, matches.length);
  const effects: StateEffect<unknown>[] = [setPageSearch.of({ query, activeIndex: normalizedIndex, activeOnly })];
  if (activeOnly && matches[normalizedIndex]) {
    effects.push(EditorView.scrollIntoView(matches[normalizedIndex].from, { y: 'center' }));
  }
  view.dispatch({ effects });
  return {
    current: matches.length ? normalizedIndex + 1 : 0,
    total: matches.length,
    activeOnly: activeOnly && matches.length > 0,
  };
}
