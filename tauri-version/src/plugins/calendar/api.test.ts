import { afterEach, expect, it, vi } from 'vitest';
import { calendarMonth, calendarOpen, dateKey, localNow } from './api';

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));
afterEach(() => { vi.useRealTimers(); invoke.mockReset(); });

it('serializes local civil time and one-based months', () => {
  const date = new Date(2026, 9, 8, 12, 34, 56);
  expect(localNow(date)).toEqual({ year: 2026, month: 10, day: 8, hour: 12, minute: 34, second: 56 });
  expect(dateKey(new Date(2026, 0, 2))).toBe('2026-01-02');
});

it('sends the Rust command contracts including local time', async () => {
  vi.useFakeTimers(); vi.setSystemTime(new Date(2026, 9, 8, 12, 34, 56));
  invoke.mockResolvedValueOnce({ weeks: [] }).mockResolvedValueOnce({ path: '/vault/2026-10-08.md', created: true });
  expect(await calendarMonth('/vault', 2026, 10)).toEqual({ weeks: [] });
  expect(invoke).toHaveBeenCalledWith('calendar_month', { workspacePath: '/vault', year: 2026, month: 10, locale: 'en', today: localNow() });
  expect(await calendarOpen('/vault', 'day', '2026-10-08', true)).toEqual({ path: '/vault/2026-10-08.md', created: true });
  expect(invoke).toHaveBeenCalledWith('calendar_open', { workspacePath: '/vault', period: 'day', date: '2026-10-08', create: true, now: localNow(), locale: 'en' });
});
