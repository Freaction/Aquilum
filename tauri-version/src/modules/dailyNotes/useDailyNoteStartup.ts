import { useEffect, useRef, useState } from 'react';
import { useStableCallback } from '../../hooks/useStableCallback';
import type { DailyNotesSettings } from './index';
import { calendarOpen, dateKey } from '../../plugins/calendar/api';

export function useDailyNoteStartup(workspacePath: string | null, ready: boolean, settings: DailyNotesSettings | undefined, onOpen: (path: string) => void): string | null {
  const activation = useRef({ workspacePath, processed: false });
  const mounted = useRef(true);
  const [error, setError] = useState<typeof activation.current | null>(null);
  const open = useStableCallback(onOpen);
  if (activation.current.workspacePath !== workspacePath) {
    activation.current = { workspacePath, processed: false };
  }

  useEffect(() => {
    mounted.current = true;
    return () => { mounted.current = false; };
  }, []);
  useEffect(() => {
    const current = activation.current;
    if (!workspacePath || !ready || !settings || current.processed) return;
    current.processed = true;
    if (!settings.openOnStartup) return;
    void calendarOpen(workspacePath, 'day', dateKey(), true)
      .then(note => { if (note && mounted.current && activation.current === current) open(note.path); })
      .catch(() => { if (mounted.current && activation.current === current) setError(current); });
  }, [workspacePath, ready, settings, open]);

  return error === activation.current ? 'calendar.startupError' : null;
}
