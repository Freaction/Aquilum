import ru from './locales/ru.json';
import en from './locales/en.json';

type Locale = 'ru' | 'en';

interface TranslationMap { [key: string]: string | TranslationMap }

const translations: Record<Locale, TranslationMap> = {
  ru: ru as TranslationMap,
  en: en as TranslationMap,
};

const STORAGE_KEY = 'aquilum_locale';

let currentLocale: Locale = 'en';

function readStoredLanguage(): string | null {
  try {
    return window.localStorage.getItem(STORAGE_KEY);
  } catch {
    return null;
  }
}

function rememberLanguage(setting: string): void {
  try {
    window.localStorage.setItem(STORAGE_KEY, setting);
  } catch {
    return;
  }
}

export function resolveLocale(setting: string | null | undefined): Locale {
  return setting === 'ru' ? 'ru' : 'en';
}

export function getLocale(): Locale {
  return currentLocale;
}

export function setLanguage(setting: string): void {
  rememberLanguage(setting);
  const next = resolveLocale(setting);
  if (next === currentLocale) return;
  currentLocale = next;
  document.documentElement.setAttribute('lang', next);
}

function lookup(locale: Locale, keys: string[]): string | null {
  let node: unknown = translations[locale];
  for (const key of keys) {
    if (!node || typeof node !== 'object' || !(key in (node as Record<string, unknown>))) return null;
    node = (node as Record<string, unknown>)[key];
  }
  return typeof node === 'string' ? node : null;
}

function interpolate(template: string, params: Record<string, string | number>): string {
  return template.replace(
    /\{\{(\w+)\}\}/g,
    (_, name: string) => (name in params ? String(params[name]) : `{{${name}}}`),
  );
}

export function t(key: string, params?: Record<string, string | number>): string {
  const keys = key.split('.');
  const translated = lookup(currentLocale, keys) ?? lookup('en', keys);
  if (translated === null) return key;
  return params ? interpolate(translated, params) : translated;
}

const pluralRules = new Map<Locale, Intl.PluralRules>();

function rulesFor(locale: Locale): Intl.PluralRules {
  const cached = pluralRules.get(locale);
  if (cached) return cached;
  const rules = new Intl.PluralRules(locale);
  pluralRules.set(locale, rules);
  return rules;
}

export function plural(key: string, count: number): string {
  const form = rulesFor(currentLocale).select(count);
  const template = lookup(currentLocale, `${key}.${form}`.split('.'))
    ?? lookup(currentLocale, `${key}.other`.split('.'))
    ?? lookup('en', `${key}.other`.split('.'));
  return template === null ? key : interpolate(template, { count });
}

export function formatNumber(value: number, fractionDigits = 0): string {
  return value.toLocaleString(currentLocale, {
    minimumFractionDigits: 0,
    maximumFractionDigits: fractionDigits,
  });
}

export function formatDateTime(epochMs: number): string {
  return new Date(epochMs).toLocaleString(currentLocale, {
    day: 'numeric',
    month: 'long',
    hour: '2-digit',
    minute: '2-digit',
  });
}

export function formatDay(epochMs: number, withYear = false): string {
  return new Date(epochMs).toLocaleDateString(currentLocale, withYear
    ? { day: 'numeric', month: 'long', year: 'numeric' }
    : { day: 'numeric', month: 'long' });
}

export function formatTime(epochMs: number): string {
  return new Date(epochMs).toLocaleTimeString(currentLocale, { hour: '2-digit', minute: '2-digit' });
}

export function initI18n(): void {
  currentLocale = resolveLocale(readStoredLanguage());
  document.documentElement.setAttribute('lang', currentLocale);
}
