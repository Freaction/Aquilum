const DEFAULT_DATE = 'YYYY-MM-DD';
const DEFAULT_TIME = 'HH:mm';

const PLACEHOLDER = /\{\{\s*(date|time)\s*(?::([^}]*))?\}\}/gi;

type Parts = Record<string, string>;

function pad(value: number, width: number): string {
  return String(value).padStart(width, '0');
}

function partsOf(moment: Date): Parts {
  return {
    YYYY: pad(moment.getFullYear(), 4),
    YY: pad(moment.getFullYear() % 100, 2),
    MMMM: moment.toLocaleDateString('ru-RU', { month: 'long' }),
    MMM: moment.toLocaleDateString('ru-RU', { month: 'short' }),
    MM: pad(moment.getMonth() + 1, 2),
    M: String(moment.getMonth() + 1),
    DDDD: moment.toLocaleDateString('ru-RU', { weekday: 'long' }),
    DDD: moment.toLocaleDateString('ru-RU', { weekday: 'short' }),
    DD: pad(moment.getDate(), 2),
    D: String(moment.getDate()),
    HH: pad(moment.getHours(), 2),
    H: String(moment.getHours()),
    mm: pad(moment.getMinutes(), 2),
    m: String(moment.getMinutes()),
    ss: pad(moment.getSeconds(), 2),
    s: String(moment.getSeconds()),
  };
}

export function formatMoment(pattern: string, moment: Date): string {
  const parts = partsOf(moment);
  const tokens = Object.keys(parts).sort((left, right) => right.length - left.length);
  const matcher = new RegExp(`\\[([^\\]]*)\\]|${tokens.join('|')}`, 'g');
  return pattern.replace(matcher, (found, literal) => literal ?? parts[found] ?? found);
}

export function applyPlaceholders(text: string, moment: Date): string {
  return text.replace(PLACEHOLDER, (_, kind: string, pattern?: string) => {
    const trimmed = pattern?.trim();
    const fallback = kind.toLowerCase() === 'date' ? DEFAULT_DATE : DEFAULT_TIME;
    return formatMoment(trimmed || fallback, moment);
  });
}
