import { useEffect, useMemo, useRef, useState } from 'react';
import type { AppConfig } from '../../modules/settings';
import { effectiveShortcut, matchesShortcut } from '../../config/shortcuts';
import { useStableCallback } from '../../hooks/useStableCallback';
import { calendarOpen, dateKey } from './api';

export function useOpenTodayNote(workspacePath: string | null, ready: boolean, enabled: boolean, onOpen: (path: string) => void, config?: AppConfig | null): string | null {
  const identity = useMemo(() => ({}), [workspacePath, ready, enabled]);
  const context = useRef<object | null>(identity);
  context.current = identity;
  const working = useRef(false);
  const [error, setError] = useState<object | null>(null);
  useEffect(() => () => { context.current = null; }, []);
  const handleKeydown = useStableCallback((event: KeyboardEvent) => {
    if (!workspacePath || !ready || !enabled || !matchesShortcut(event, effectiveShortcut(config, 'OPEN_TODAY_NOTE'))) return;
    event.preventDefault();
    if (working.current) return;
    working.current = true;
    setError(null);
    const request = identity;
    void calendarOpen(workspacePath, 'day', dateKey(), true)
      .then(note => { if (note && context.current === request) onOpen(note.path); })
      .catch(() => { if (context.current === request) setError(request); })
      .finally(() => { working.current = false; });
  });
  useEffect(() => {
    window.addEventListener('keydown', handleKeydown);
    return () => window.removeEventListener('keydown', handleKeydown);
  }, [handleKeydown]);
  return error === identity ? 'plugins.calendar.openTodayError' : null;
}
