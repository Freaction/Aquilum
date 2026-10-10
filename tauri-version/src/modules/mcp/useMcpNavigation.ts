import { useEffect } from 'react';
import { useTauriEvent } from '../../hooks/useTauriEvent';
import type { LinkDisposition } from '../links';
import { isEmptyTabPath } from '../ui-state';
import { setActiveNote } from '.';

const MCP_NAVIGATE_EVENT = 'mcp-navigate';

interface NavigatePayload {
  kind: 'note' | 'workspace';
  path: string;
  disposition?: LinkDisposition;
}

interface McpNavigation {
  openNote: (path: string, disposition: LinkDisposition) => void;
  openWorkspace: (path: string) => void;
}

export function useActiveNoteReport(activeFile: string | null): void {
  useEffect(() => {
    const path = activeFile && !isEmptyTabPath(activeFile) ? activeFile : null;
    void setActiveNote(path).catch((error) => {
      console.error('Failed to report active note', error);
    });
  }, [activeFile]);
}

export function useMcpNavigation(navigation: McpNavigation): void {
  useTauriEvent<NavigatePayload>(MCP_NAVIGATE_EVENT, ({ kind, path, disposition }) => {
    if (kind === 'note') navigation.openNote(path, disposition ?? 'current');
    else if (kind === 'workspace') navigation.openWorkspace(path);
  });
}
