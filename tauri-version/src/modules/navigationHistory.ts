export interface NavigationHistoryState {
  entries: string[];
  index: number;
}

const MAX_ENTRIES = 100;

export const emptyNavigationHistory: NavigationHistoryState = {
  entries: [],
  index: -1,
};

export function pushNavigationEntry(
  state: NavigationHistoryState,
  path: string,
): NavigationHistoryState {
  if (state.entries[state.index] === path) return state;
  const entries = state.entries.slice(0, state.index + 1);
  entries.push(path);
  if (entries.length > MAX_ENTRIES) {
    entries.splice(0, entries.length - MAX_ENTRIES);
  }
  return { entries, index: entries.length - 1 };
}

export function stepNavigationHistory(
  state: NavigationHistoryState,
  delta: -1 | 1,
): { state: NavigationHistoryState; path: string | null } {
  const nextIndex = state.index + delta;
  if (nextIndex < 0 || nextIndex >= state.entries.length) {
    return { state, path: null };
  }
  return {
    state: { ...state, index: nextIndex },
    path: state.entries[nextIndex] ?? null,
  };
}

export function canNavigateBack(state: NavigationHistoryState): boolean {
  return state.index > 0;
}

export function canNavigateForward(state: NavigationHistoryState): boolean {
  return state.index >= 0 && state.index < state.entries.length - 1;
}
