export function nextLiveTabs(
  previous: readonly string[],
  activeTabId: string | null,
  keepable: (tabId: string) => boolean,
  limit: number,
): string[] {
  const ordered = activeTabId === null
    ? [...previous]
    : [activeTabId, ...previous.filter((tabId) => tabId !== activeTabId)];
  return ordered.filter(keepable).slice(0, Math.max(1, Math.floor(limit)));
}

export function sameTabOrder(left: readonly string[], right: readonly string[]): boolean {
  return left.length === right.length && left.every((tabId, index) => tabId === right[index]);
}
