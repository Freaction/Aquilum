import { useRef } from 'react';
import { useTauriEvent } from '../../hooks/useTauriEvent';
import {
  applyRelocation,
  NOTES_RELOCATED_EVENT,
  type NotesRelocated,
  type RelocationTargets,
} from './relocation';

export function useNoteRelocation(targets: RelocationTargets): void {
  const targetsRef = useRef(targets);
  targetsRef.current = targets;

  useTauriEvent<NotesRelocated>(NOTES_RELOCATED_EVENT, (relocation) => {
    applyRelocation(relocation, targetsRef.current);
  });
}
