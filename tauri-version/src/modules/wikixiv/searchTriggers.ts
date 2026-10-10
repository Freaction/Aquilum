export const WIKIXIV_PAUSE_MS = 6000;

export function looksLikeParagraphBreak(previous: string, next: string): boolean {
  if (next.length <= previous.length) return false;
  const added = next.slice(Math.max(0, previous.length - 2));
  return /\n\s*\n/.test(added) || /(^|\n)#{1,6}\s.+\n/.test(added);
}
