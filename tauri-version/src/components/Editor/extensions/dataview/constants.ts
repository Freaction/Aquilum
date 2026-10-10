import type { PrefetchDelays } from '../prefetchScheduler';

export const DATAVIEW_LANGUAGE = 'dataview';

export const MAX_CACHED_NOTES = 8;
export const UNKNOWN_HEIGHT = -1;
export const PENDING_HEIGHT = '96px';


export const DATAVIEW_PREFETCH_DELAYS: PrefetchDelays = {
  editMs: 250,
  revisionMs: 0,
};

export const DATAVIEW_CLASSES = {
  root: 'q-md-dataview',
  card: 'q-md-dataview-card',
  pending: 'q-md-dataview-card--pending',
  failed: 'q-md-dataview-card--failed',
  title: 'q-md-dataview-title',
  titleCells: 'q-md-dataview-title--cells',
  table: 'q-md-dataview-table',
  tasks: 'q-md-dataview-tasks',
  list: 'q-md-dataview-list',
  link: 'q-md-dataview-link',
  cellBar: 'q-md-dataview-cell-bar',
  bar: 'q-md-dataview-bar',
  barFill: 'q-md-dataview-bar-fill',
  message: 'q-md-dataview-message',
} as const;

export const DATAVIEW_SELECTORS = {
  card: `.${DATAVIEW_CLASSES.card}`,
} as const;

export const DATAVIEW_MESSAGES = {
  empty: 'Запрос ничего не нашёл',
  truncated: (count: number, total: number) => `Показаны первые ${count} из ${total}`,
} as const;
