interface WikiHoverState {
  filename: boolean;
  terms: string[];
}

const EMPTY: WikiHoverState = { filename: false, terms: [] };

let state: WikiHoverState = EMPTY;
const listeners = new Set<() => void>();

export function setWikiHover(next: WikiHoverState | null): void {
  const value = next ?? EMPTY;
  if (value === state) return;
  state = value;
  for (const listener of listeners) listener();
}

export function clearWikiHover(): void {
  setWikiHover(null);
}

export function getWikiHover(): WikiHoverState {
  return state;
}

export function subscribeWikiHover(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}
