import { invoke } from '@tauri-apps/api/core';
import { getLocale } from '../../i18n';

export interface LocalDateTime {
  year: number;
  month: number;
  day: number;
  hour: number;
  minute: number;
  second: number;
}

export interface CalendarDay {
  date: string;
  inMonth: boolean;
  isToday: boolean;
  path: string;
  exists: boolean;
  words: number;
  dots: number;
  openTasks: boolean;
}

export interface CalendarWeek {
  number: number;
  key: string;
  path: string | null;
  exists: boolean;
  days: CalendarDay[];
}

export interface CalendarMonth { weeks: CalendarWeek[]; }
export interface OpenedNote { path: string; created: boolean; }
export type CalendarPeriod = 'day' | 'week';

export function localNow(date = new Date()): LocalDateTime {
  return {
    year: date.getFullYear(), month: date.getMonth() + 1, day: date.getDate(),
    hour: date.getHours(), minute: date.getMinutes(), second: date.getSeconds(),
  };
}

export function dateKey(date = new Date()): string {
  const { year, month, day } = localNow(date);
  return `${String(year).padStart(4, '0')}-${String(month).padStart(2, '0')}-${String(day).padStart(2, '0')}`;
}

export function calendarMonth(workspacePath: string, year: number, month: number): Promise<CalendarMonth> {
  return invoke('calendar_month', { workspacePath, year, month, locale: getLocale(), today: localNow() });
}

export function calendarOpen(workspacePath: string, period: CalendarPeriod, date: string, create: boolean): Promise<OpenedNote | null> {
  return invoke('calendar_open', { workspacePath, period, date, create, now: localNow(), locale: getLocale() });
}
