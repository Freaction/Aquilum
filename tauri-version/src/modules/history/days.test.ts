import { describe, expect, it } from 'vitest';
import { groupByDay } from './days';
import type { NoteVersion } from './index';

function version(id: string, date: Date): NoteVersion {
  return { id, atMs: date.getTime(), device: 'aaaaaaaa', source: 'me', fromMs: null, name: null, isCurrent: false };
}

describe('groupByDay', () => {
  it('keeps the newest-first order and starts a group on every new day', () => {
    const now = new Date(2026, 8, 28, 15, 0);
    const groups = groupByDay([
      version('c', new Date(2026, 8, 28, 14, 30)),
      version('b', new Date(2026, 8, 28, 9, 0)),
      version('a', new Date(2026, 8, 27, 23, 59)),
      version('z', new Date(2026, 8, 20, 12, 0)),
    ], now.getTime());

    expect(groups.map((group) => group.versions.map((item) => item.id))).toEqual([['c', 'b'], ['a'], ['z']]);
    expect(groups[0].label).not.toBe(groups[1].label);
    expect(groups[2].label).not.toBe(groups[1].label);
  });

  it('names a day of another year with the year', () => {
    const [group] = groupByDay([version('a', new Date(2025, 0, 5, 10, 0))], new Date(2026, 8, 28).getTime());
    expect(group.label).toMatch(/2025/);
  });
});
