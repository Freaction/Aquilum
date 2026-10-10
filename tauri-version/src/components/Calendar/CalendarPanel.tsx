import { useEffect, useMemo, useRef, useState, type KeyboardEvent } from 'react';
import { ChevronLeft, ChevronRight } from 'lucide';
import { getLocale, t } from '../../i18n';
import { useTauriEvent } from '../../hooks/useTauriEvent';
import { samePath } from '../../modules/paths';
import type { DailyNotesSettings } from '../../modules/dailyNotes';
import { DEFAULT_PLUGIN_SETTINGS, type PluginSettings } from '../../modules/settings';
import { calendarMonth, calendarOpen, dateKey, type CalendarMonth, type CalendarPeriod } from '../../plugins/calendar/api';
import { Icon } from '../Common/Icon';
import { IconButton } from '../Common/IconButton';
import { TextButton } from '../Common/TextButton';
import { ConfirmDialog } from '../Common/ConfirmDialog';
import './CalendarPanel.css';

interface Props {
  workspacePath: string | null;
  documentPath: string | null;
  settings: DailyNotesSettings;
  calendarSettings?: PluginSettings['calendar'];
  indexRevision?: number;
  onOpenNote: (path: string) => void;
}
interface Pending { period: CalendarPeriod; date: string; }
const asDate = (date: string) => new Date(`${date}T12:00:00`);

export function CalendarPanel({ workspacePath, documentPath, settings, calendarSettings = DEFAULT_PLUGIN_SETTINGS.calendar, indexRevision = 0, onOpenNote }: Props) {
  const [month, setMonth] = useState(() => new Date());
  const [today, setToday] = useState(() => dateKey());
  const [data, setData] = useState<CalendarMonth>({ weeks: [] });
  const [revision, refresh] = useState(0);
  const [error, setError] = useState(false);
  const [busy, setBusy] = useState(false);
  const [loading, setLoading] = useState(false);
  const [pending, setPending] = useState<Pending | null>(null);
  const working = useRef(false);
  const locale = getLocale();
  const identity = useMemo(() => ({}), [workspacePath, settings.folder, settings.format, settings.template,
    settings.weekStart, calendarSettings.weekly.enabled, calendarSettings.weekly.folder,
    calendarSettings.weekly.format, calendarSettings.weekly.template, locale]);
  const context = useRef<object | null>(identity);
  context.current = identity;
  useEffect(() => () => { context.current = null; }, []);
  const grid = useRef<HTMLDivElement>(null);
  const focusTarget = useRef<Pending | null>(null);
  const weekStart = settings.weekStart === 'monday' || (settings.weekStart === 'locale' && locale === 'ru') ? 1 : 0;
  const reload = () => refresh(value => value + 1);
  useTauriEvent('document-saved', reload);
  useTauriEvent('workspace-changed', reload);

  useEffect(() => {
    const timer = window.setInterval(() => setToday(dateKey()), 30_000);
    return () => window.clearInterval(timer);
  }, []);
  useEffect(() => {
    let active = true;
    setData({ weeks: [] }); setError(false); setLoading(Boolean(workspacePath));
    if (workspacePath) {
      void calendarMonth(workspacePath, month.getFullYear(), month.getMonth() + 1)
        .then(result => { if (active) setData(result); })
        .catch(() => { if (active) setError(true); })
        .finally(() => { if (active) setLoading(false); });
    }
    return () => { active = false; };
  }, [identity, month, today, settings.wordsPerDot, revision, indexRevision]);
  useEffect(() => { setPending(null); }, [identity]);
  useEffect(() => {
    if (focusTarget.current) {
      const { period, date } = focusTarget.current;
      const button = grid.current?.querySelector<HTMLButtonElement>(`[data-${period === 'day' ? 'date' : 'week'}="${date}"]`);
      if (button) { button.focus(); focusTarget.current = null; }
    }
  }, [data, pending]);

  async function open(period: CalendarPeriod, date: string, confirmed = false) {
    if (!workspacePath || working.current) return;
    const workspace = workspacePath;
    const requestContext = identity;
    working.current = true; setBusy(true); setError(false);
    try {
      const existing = await calendarOpen(workspace, period, date, false);
      if (context.current !== requestContext) return;
      if (!existing && settings.confirmBeforeCreate && !confirmed) { setPending({ period, date }); return; }
      const note = existing ?? await calendarOpen(workspace, period, date, true);
      if (context.current !== requestContext || !note) return;
      setPending(null); reload(); onOpenNote(note.path);
    } catch { if (context.current === requestContext) setError(true); }
    finally { working.current = false; setBusy(false); }
  }
  function moveMonth(delta: number) { setMonth(value => new Date(value.getFullYear(), value.getMonth() + delta, 1, 12)); }
  function onDayKey(event: KeyboardEvent<HTMLButtonElement>, day: Date) {
    const offsets: Record<string, number> = { ArrowLeft: -1, ArrowRight: 1, ArrowUp: -7, ArrowDown: 7, Home: -((day.getDay() - weekStart + 7) % 7), End: 6 - ((day.getDay() - weekStart + 7) % 7) };
    const next = new Date(day);
    if (event.key in offsets) next.setDate(next.getDate() + offsets[event.key]);
    else if (event.key === 'PageUp' || event.key === 'PageDown') {
      next.setDate(1); next.setMonth(next.getMonth() + (event.key === 'PageUp' ? -1 : 1));
      next.setDate(Math.min(day.getDate(), new Date(next.getFullYear(), next.getMonth() + 1, 0).getDate()));
    } else return;
    event.preventDefault();
    const key = dateKey(next);
    const button = grid.current?.querySelector<HTMLButtonElement>(`[data-date="${key}"]`);
    if (button) button.focus();
    else { focusTarget.current = { period: 'day', date: key }; setMonth(next); }
  }
  const cancel = () => { focusTarget.current = pending; setPending(null); };
  const firstWeek = data.weeks[0];
  const weekLabel = (number: number) => t('plugins.calendar.week', { number });
  return <section className="q-calendar" aria-label={t('calendar.title')} onKeyDown={event => {
    if (!pending || event.key !== 'Tab') return;
    const dialog = event.currentTarget.querySelector('[role="alertdialog"]');
    const buttons = dialog?.querySelectorAll<HTMLButtonElement>('button:not(:disabled)');
    if (!buttons?.length) { event.preventDefault(); return; }
    const first = buttons[0], last = buttons[buttons.length - 1];
    if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last.focus(); }
    else if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first.focus(); }
  }}>
    <div className="q-calendar__navigation">
      <IconButton label={t('calendar.previous')} onClick={() => moveMonth(-1)}><Icon icon={ChevronLeft} /></IconButton>
      <h2 aria-live="polite">{month.toLocaleDateString(locale, { month: 'long', year: 'numeric' })}</h2>
      <IconButton label={t('calendar.next')} onClick={() => moveMonth(1)}><Icon icon={ChevronRight} /></IconButton>
    </div>
    <TextButton onClick={() => setMonth(new Date())}>{t('calendar.today')}</TextButton>
    {!workspacePath ? <p>{t('calendar.noWorkspace')}</p> : <>
      <div className={`q-calendar__grid${calendarSettings.showWeekNumbers ? ' q-calendar__grid--weeks' : ''}`} ref={grid} aria-busy={busy || loading}>
        {calendarSettings.showWeekNumbers && <abbr className="q-calendar__weekday q-calendar__week-heading" title={t('plugins.calendar.weekNumbers')} aria-label={t('plugins.calendar.weekNumbers')}>{t('plugins.calendar.weekNumbersShort')}</abbr>}
        {firstWeek?.days.map(day => {
          const date = asDate(day.date);
          return <abbr className="q-calendar__weekday" key={day.date} title={date.toLocaleDateString(locale, { weekday: 'long' })}>{date.toLocaleDateString(locale, { weekday: 'short' })}</abbr>;
        })}
        {data.weeks.map(week => <div className="q-calendar__week" key={week.key}>
          {calendarSettings.showWeekNumbers && (calendarSettings.weekly.enabled
            ? <button type="button" className="q-calendar__day q-calendar__week-number" data-week={week.key} aria-label={weekLabel(week.number)} title={weekLabel(week.number)} aria-pressed={samePath(documentPath, week.path)} disabled={busy} onClick={() => void open('week', week.key)}>{week.number}</button>
            : <span className="q-calendar__weekday q-calendar__week-number" aria-label={weekLabel(week.number)}>{week.number}</span>)}
          {week.days.map(day => {
            const date = asDate(day.date);
            const selected = samePath(documentPath, day.path);
            const label = `${date.toLocaleDateString(locale, { dateStyle: 'full' })}${day.exists ? `, ${t('calendar.words', { count: day.words })}${day.openTasks ? `, ${t('calendar.tasks')}` : ''}` : ''}`;
            return <button type="button" key={day.date} data-date={day.date} className={`q-calendar__day${!day.inMonth ? ' is-outside' : ''}${selected ? ' is-selected' : ''}`} aria-label={label} title={label} aria-current={day.isToday ? 'date' : undefined} aria-pressed={selected} disabled={busy} onKeyDown={event => onDayKey(event, date)} onClick={() => void open('day', day.date)}>
              <span>{date.getDate()}</span>
              <span className="q-calendar__dots" aria-hidden="true">{Array.from({ length: day.dots }, (_, i) => <i key={i} className="q-calendar__dot" />)}{day.openTasks && <i className="q-calendar__dot is-hollow" />}</span>
            </button>;
          })}
        </div>)}
      </div>
      {error && <p role="alert">{t('calendar.error')} <TextButton onClick={reload}>{t('calendar.retry')}</TextButton></p>}
    </>}
    <ConfirmDialog open={pending !== null} title={t(pending?.period === 'week' ? 'plugins.calendar.createWeekTitle' : 'calendar.createTitle')} description={error ? t('calendar.error') : t(pending?.period === 'week' ? 'plugins.calendar.createWeekDescription' : 'calendar.createDescription', { date: pending ? asDate(pending.date).toLocaleDateString(locale, { dateStyle: 'long' }) : '' })} confirmLabel={t('calendar.create')} pendingLabel={t('calendar.creating')} pending={busy} onCancel={cancel} onConfirm={() => { if (pending) void open(pending.period, pending.date, true); }} />
  </section>;
}
