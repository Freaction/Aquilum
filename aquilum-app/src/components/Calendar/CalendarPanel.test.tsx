// @vitest-environment happy-dom
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { actAndSettle, mountDom, type MountedDom } from '../../testing/mountDom';
import { DEFAULT_DAILY_NOTES_SETTINGS } from '../../modules/dailyNotes';
import { CalendarPanel } from './CalendarPanel';
import { setLanguage, t } from '../../i18n';
import { DEFAULT_PLUGIN_SETTINGS } from '../../modules/settings';

const { invoke, listeners } = vi.hoisted(() => ({ invoke: vi.fn(), listeners: new Map<string, (event: { payload: unknown }) => void>() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(async (name, callback) => { listeners.set(name, callback); return () => listeners.delete(name); }) }));
let mounted: MountedDom;
const files = new Map<string, string>();
const open = vi.fn();
beforeEach(() => {
  files.clear(); open.mockReset(); invoke.mockReset(); listeners.clear();
  invoke.mockImplementation(async (command, args) => {
    if (command === 'calendar_month') {
      const first = new Date(args.year, args.month - 1, 1, 12);
      first.setDate(first.getDate() - first.getDay());
      const last = new Date(args.year, args.month, 0, 12);
      last.setDate(last.getDate() + 6 - last.getDay());
      const weeks = [];
      for (const day = new Date(first); day <= last;) {
        const days = [];
        for (let i = 0; i < 7; i++, day.setDate(day.getDate() + 1)) {
          const date = key(day), path = `${args.workspacePath}/${date}.md`;
          const content = files.get(path), words = content?.trim().split(/\s+/).filter(Boolean).length ?? 0;
          days.push({ date, path, inMonth: day.getMonth() === args.month - 1,
            isToday: date === key(new Date()), exists: content !== undefined, words,
            dots: words > 0 ? Math.min(5, Math.max(1, Math.floor(words / 250))) : 0,
            openTasks: content?.includes('- [ ]') ?? false });
        }
        const weekKey = days[0].date, path = `${args.workspacePath}/weekly-${weekKey}.md`;
        weeks.push({ key: weekKey, number: weeks.length + 1, path, exists: files.has(path), days });
      }
      return { weeks };
    }
    if (command === 'calendar_open') {
      const path = args.period === 'week' ? `${args.workspacePath}/weekly-${args.date}.md` : `${args.workspacePath}/${args.date}.md`;
      const exists = files.has(path);
      if (!exists && !args.create) return null;
      if (!exists) files.set(path, '');
      return { path, created: !exists };
    }
    throw new Error(`Unexpected command: ${command}`);
  });
});
afterEach(() => { mounted?.unmount(); setLanguage('en'); });
async function render(confirmBeforeCreate = false) {
  await actAndSettle(() => { mounted = mountDom(<CalendarPanel workspacePath="/vault" documentPath={null} settings={{ ...DEFAULT_DAILY_NOTES_SETTINGS, confirmBeforeCreate }} onOpenNote={open} />); });
}
it('refreshes external content changes when the workspace index advances', async () => {
  await render();
  await actAndSettle(() => today().click());
  const path = open.mock.calls[0][0];
  files.set(path, 'word '.repeat(750));
  await actAndSettle(() => mounted.update(<CalendarPanel workspacePath="/vault" documentPath={path} settings={DEFAULT_DAILY_NOTES_SETTINGS} onOpenNote={open} indexRevision={1} />));
  expect(today().querySelectorAll('.q-calendar__dot:not(.is-hollow)')).toHaveLength(3);
});
function key(date: Date) {
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`;
}
function today() { return mounted.container.querySelector<HTMLButtonElement>('[aria-current="date"]')!; }
it('creates a daily note once and refreshes saved word indicators', async () => {
  await render();
  await actAndSettle(() => today().click());
  expect(open).toHaveBeenCalledTimes(1);
  const path = open.mock.calls[0][0];
  expect(files.get(path)).toBe('');
  await actAndSettle(() => today().click());
  expect(invoke.mock.calls.filter(([command, args]) => command === 'calendar_open' && args.create)).toHaveLength(1);
  files.set(path, 'word '.repeat(500));
  await actAndSettle(() => listeners.get('document-saved')?.({ payload: { path } }));
  expect(today().querySelectorAll('.q-calendar__dot:not(.is-hollow)')).toHaveLength(2);
});
it('asks before creating a missing daily and cancellation writes nothing', async () => {
  await render(true);
  await actAndSettle(() => today().click());
  expect(document.querySelector('[role="alertdialog"]')).not.toBeNull();
  expect(files.size).toBe(0);
  await actAndSettle(() => { document.querySelector('[role="alertdialog"]')!.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true })); });
  expect(files.size).toBe(0);
  expect(open).not.toHaveBeenCalled();
  expect(document.querySelector('[role="alertdialog"]')).toBeNull();
  expect(document.activeElement).toBe(today());
});
it('shows read errors and never creates a file after an I/O failure', async () => {
  const original = invoke.getMockImplementation()!;
  invoke.mockImplementation((command, args) => command === 'calendar_open' ? Promise.reject({ code: 'io', details: { message: 'denied' } }) : original(command, args));
  await render();
  await actAndSettle(() => today().click());
  expect(mounted.container.querySelector('[role="alert"]')).not.toBeNull();
  expect(invoke.mock.calls.some(([command, args]) => command === 'calendar_open' && args.create)).toBe(false);
  expect(open).not.toHaveBeenCalled();
});

it('creates the note after confirmation and closes the dialog', async () => {
  await render(true);
  await actAndSettle(() => today().click());
  expect(open).not.toHaveBeenCalled();
  const button = Array.from(document.querySelectorAll<HTMLButtonElement>('[role="alertdialog"] button')).find(button => button.textContent === t('calendar.create'))!;
  await actAndSettle(() => new Promise<void>(resolve => requestAnimationFrame(() => resolve())));
  expect(document.activeElement).toBe(button);
  await actAndSettle(() => button.click());
  expect(open).toHaveBeenCalledTimes(1);
  expect(files.get(open.mock.calls[0][0])).toBe('');
  expect(document.querySelector('[role="alertdialog"]')).toBeNull();
});

it('moves keyboard focus across dates and month boundaries', async () => {
  await render();
  const first = mounted.container.querySelector<HTMLButtonElement>('[data-date]')!;
  const firstDate = first.dataset.date!;
  await actAndSettle(() => {
    first.focus();
    first.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowLeft', bubbles: true }));
  });
  const target = new Date(firstDate + 'T12:00:00');
  target.setDate(target.getDate() - 1);
  const expected = target.getFullYear() + '-' + String(target.getMonth() + 1).padStart(2, '0') + '-' + String(target.getDate()).padStart(2, '0');
  expect((document.activeElement as HTMLElement).dataset.date).toBe(expected);
  await actAndSettle(() => {
    document.activeElement!.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true }));
  });
  target.setDate(target.getDate() + 7);
  const after = target.getFullYear() + '-' + String(target.getMonth() + 1).padStart(2, '0') + '-' + String(target.getDate()).padStart(2, '0');
  expect((document.activeElement as HTMLElement).dataset.date).toBe(after);
});

it.each(['workspace roundtrip', 'settings change'])('does not open stale pending reads after %s', async change => {
  await render();
  const original = invoke.getMockImplementation()!;
  let resolve!: (value: { path: string; created: boolean }) => void;
  let blocked = false;
  invoke.mockImplementation((command, args) => {
    if (command === 'calendar_open' && !blocked) {
      blocked = true;
      return new Promise(done => { resolve = done; });
    }
    return original(command, args);
  });
  await actAndSettle(() => today().click());
  const panel = (workspacePath: string, folder = '') => <CalendarPanel workspacePath={workspacePath} documentPath={null} settings={{ ...DEFAULT_DAILY_NOTES_SETTINGS, folder, confirmBeforeCreate: false }} onOpenNote={open} />;
  if (change === 'workspace roundtrip') {
    await actAndSettle(() => mounted.update(panel('/second')));
    await actAndSettle(() => mounted.update(panel('/vault')));
  } else await actAndSettle(() => mounted.update(panel('/vault', 'Daily')));
  await actAndSettle(() => resolve({ path: '/vault/old.md', created: false }));
  expect(open).not.toHaveBeenCalled();
  expect(files.size).toBe(0);
  expect(mounted.container.querySelector('[role="alert"]')).toBeNull();
});

it('shows week numbers and opens a weekly note through the command', async () => {
  const calendarSettings = { ...DEFAULT_PLUGIN_SETTINGS.calendar, showWeekNumbers: true,
    weekly: { ...DEFAULT_PLUGIN_SETTINGS.calendar.weekly, enabled: true } };
  await actAndSettle(() => { mounted = mountDom(<CalendarPanel workspacePath="/vault" documentPath={null} settings={{ ...DEFAULT_DAILY_NOTES_SETTINGS, confirmBeforeCreate: false }} calendarSettings={calendarSettings} onOpenNote={open} />); });
  const week = mounted.container.querySelector<HTMLButtonElement>('button[data-week]')!;
  expect(mounted.container.querySelector('.q-calendar__week-heading')?.textContent).toBe('Wk');
  expect(mounted.container.querySelector('.q-calendar__week-heading')?.getAttribute('title')).toBe(t('plugins.calendar.weekNumbers'));
  expect(mounted.container.querySelector('.q-calendar__week-heading')?.getAttribute('aria-label')).toBe(t('plugins.calendar.weekNumbers'));
  await actAndSettle(() => week.click());
  expect(open).toHaveBeenCalledWith(`/vault/weekly-${week.dataset.week}.md`);
  expect(invoke).toHaveBeenCalledWith('calendar_open', expect.objectContaining({ period: 'week', create: true, date: week.dataset.week }));
});

it('keeps week numbers noninteractive when weekly notes are disabled', async () => {
  await actAndSettle(() => { mounted = mountDom(<CalendarPanel workspacePath="/vault" documentPath={null} settings={DEFAULT_DAILY_NOTES_SETTINGS} calendarSettings={{ ...DEFAULT_PLUGIN_SETTINGS.calendar, showWeekNumbers: true }} onOpenNote={open} />); });
  expect(mounted.container.querySelectorAll('.q-calendar__week-number').length).toBeGreaterThan(0);
  expect(mounted.container.querySelector('button[data-week]')).toBeNull();
});

it.each([['en', 'Create weekly note?', 'Wk'], ['ru', 'Создать еженедельную заметку?', 'Нед']])('confirms weekly creation in %s and returns focus on cancellation', async (locale, title, heading) => {
  setLanguage(locale);
  await actAndSettle(() => { mounted = mountDom(<CalendarPanel workspacePath="/vault" documentPath={null} settings={DEFAULT_DAILY_NOTES_SETTINGS} calendarSettings={{ ...DEFAULT_PLUGIN_SETTINGS.calendar, showWeekNumbers: true, weekly: { ...DEFAULT_PLUGIN_SETTINGS.calendar.weekly, enabled: true } }} onOpenNote={open} />); });
  const week = mounted.container.querySelector<HTMLButtonElement>('button[data-week]')!;
  await actAndSettle(() => week.click());
  expect(document.querySelector('[role="alertdialog"]')).not.toBeNull();
  expect(document.querySelector('[role="alertdialog"] .q-dialog__title')?.textContent).toBe(title);
  expect(mounted.container.querySelector('.q-calendar__week-heading')?.textContent).toBe(heading);
  expect(files.size).toBe(0);
  await actAndSettle(() => { document.querySelector('[role="alertdialog"]')!.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true })); });
  expect(document.activeElement).toBe(week);
  expect(open).not.toHaveBeenCalled();
});
