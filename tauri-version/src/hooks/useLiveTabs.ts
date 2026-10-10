import { useEffect, useState } from 'react';
import { nextLiveTabs, sameTabOrder } from '../modules/liveTabs';
import type { SessionTab } from '../modules/ui-state';

export function useLiveTabs(
  tabs: SessionTab[],
  activeTabId: string | null,
  limit: number,
): string[] {
  const [live, setLive] = useState<string[]>([]);

  useEffect(() => {
    const keepable = (tabId: string) => tabs.some(
      (tab) => tab.tabId === tabId && tab.kind === 'document',
    );
    setLive((previous) => {
      const next = nextLiveTabs(previous, activeTabId, keepable, limit);
      return sameTabOrder(next, previous) ? previous : next;
    });
  }, [activeTabId, limit, tabs]);

  return live;
}
