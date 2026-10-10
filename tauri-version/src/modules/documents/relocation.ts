export const NOTES_RELOCATED_EVENT = 'notes-relocated';

export interface NotesRelocated {
  moves: { from: string; to: string }[];
  removed: string[];
}

export interface RelocationTargets {
  renamed: (from: string, to: string) => void;
  deleted: (path: string) => void;
}

export function applyRelocation(event: NotesRelocated, targets: RelocationTargets): void {
  for (const { from, to } of event.moves) targets.renamed(from, to);
  for (const path of event.removed) targets.deleted(path);
}
