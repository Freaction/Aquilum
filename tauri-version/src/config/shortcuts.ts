import type { AppConfig } from '../modules/settings';
import { isMacOs } from '../modules/platform';

export interface ShortcutToken {
  code?: string;
  key: string;
  display?: string;
  primary?: boolean;
  shift?: boolean;
  alt?: boolean;
}

export const SHORTCUTS = {
  NEW_FILE: {
    code: 'KeyN',
    key: 'N',
    primary: true,
  },
  NEW_TAB: {
    code: 'KeyT',
    key: 'T',
    primary: true,
  },
  GLOBAL_SEARCH: {
    code: 'KeyO',
    key: 'O',
    primary: true,
  },
  NEW_FROM_TEMPLATE: {
    code: 'KeyU',
    key: 'U',
    primary: true,
  },
  FOCUS_MODE: {
    code: 'KeyF',
    key: 'F',
    primary: true,
    shift: true,
  },
  PAGE_SEARCH: {
    code: 'KeyF',
    key: 'F',
    primary: true,
  },
  ZOOM_IN: {
    key: '=',
    display: '+',
    primary: true,
  },
  ZOOM_OUT: {
    key: '-',
    primary: true,
  },
  ZOOM_RESET: {
    key: '0',
    primary: true,
  },
  BOLD: {
    key: 'B',
    primary: true,
  },
  ITALIC: {
    key: 'I',
    primary: true,
  },
  STRIKETHROUGH: {
    key: 'S',
    primary: true,
    shift: true,
  },
  INDENT: {
    key: 'Tab',
  },
  OUTDENT: {
    key: 'Tab',
    shift: true,
  },
  LINE_BREAK: {
    key: 'Enter',
    display: '↵',
    shift: true,
  },
  PAGE_SEARCH_NEXT: {
    key: 'Enter',
    display: '↵',
  },
  PAGE_SEARCH_PREVIOUS: {
    key: 'Enter',
    display: '↵',
    shift: true,
  },
  READER_NEXT_PAGE: {
    key: 'ArrowRight',
    display: '→',
  },
  READER_PREVIOUS_PAGE: {
    key: 'ArrowLeft',
    display: '←',
  },
  SEARCH_PREVIOUS: {
    key: 'ArrowUp',
    display: '↑',
  },
  SEARCH_NEXT: {
    key: 'ArrowDown',
    display: '↓',
  },
  SEARCH_OPEN: {
    key: 'Enter',
    display: '↵',
  },
  CLOSE_DIALOG: {
    key: 'Escape',
    display: 'Esc',
  },
  SEARCH_OPEN_NEW_PANE: {
    key: 'Enter',
    display: '↵',
    primary: true,
  },
  SEARCH_CREATE: {
    key: 'Enter',
    display: '↵',
    shift: true,
  },
  OPEN_TODAY_NOTE: { code: 'KeyD', key: 'D', primary: true, shift: true },
  TOGGLE_FULL_WIDTH: { code: 'KeyW', key: 'W', primary: true, alt: true },
  TOGGLE_READING_MODE: { code: 'KeyE', key: 'E', primary: true, shift: true },
} as const satisfies Record<string, ShortcutToken>;

export const ZOOM_IN_ALIASES: readonly ShortcutToken[] = [
  { key: '+', primary: true },
  { key: '+', primary: true, shift: true },
];

export function matchesShortcut(event: KeyboardEvent, shortcut: ShortcutToken): boolean {
  const hasPrimaryModifier = event.ctrlKey || event.metaKey;
  const keyMatches = shortcut.code
    ? event.code === shortcut.code
    : event.key === shortcut.key;
  return keyMatches
    && hasPrimaryModifier === Boolean(shortcut.primary)
    && event.altKey === Boolean(shortcut.alt)
    && event.shiftKey === Boolean(shortcut.shift);
}

export function codeMirrorKey(shortcut: ShortcutToken): string {
  const keys = [];
  if (shortcut.primary) keys.push('Mod');
  if (shortcut.alt) keys.push('Alt');
  if (shortcut.shift) keys.push('Shift');
  keys.push(shortcut.key.length === 1 ? shortcut.key.toLowerCase() : shortcut.key);
  return keys.join('-');
}

export function shortcutModifiers(shortcut: ShortcutToken): string[] {
  const keys = [];
  if (shortcut.primary) keys.push('Ctrl');
  if (shortcut.shift) keys.push('Shift');
  if (shortcut.alt) keys.push('Alt');
  return keys;
}

export function formatShortcut(shortcut: ShortcutToken, separator = ' + '): string {
  const key = shortcut.display ?? shortcut.key;
  if (isMacOs()) {
    return `${shortcut.primary ? '⌘' : ''}${shortcut.alt ? '⌥' : ''}${shortcut.shift ? '⇧' : ''}${key}`;
  }
  return [...shortcutModifiers(shortcut), key].join(separator);
}

export type ConfigurableShortcut = 'OPEN_TODAY_NOTE' | 'TOGGLE_READING_MODE' | 'TOGGLE_FULL_WIDTH';

export function effectiveShortcut(config: AppConfig | null | undefined, id: keyof typeof SHORTCUTS): ShortcutToken {
  if (id === 'OPEN_TODAY_NOTE') return config?.plugins?.calendar.openTodayShortcut ?? SHORTCUTS[id];
  if (id === 'TOGGLE_READING_MODE') return config?.plugins?.readingMode.shortcut ?? SHORTCUTS[id];
  if (id === 'TOGGLE_FULL_WIDTH') return config?.editor.fullWidthShortcut ?? SHORTCUTS[id];
  return SHORTCUTS[id];
}

export function shortcutConflict(config: AppConfig, id: ConfigurableShortcut, candidate: ShortcutToken): keyof typeof SHORTCUTS | undefined {
  return (Object.keys(SHORTCUTS) as (keyof typeof SHORTCUTS)[]).find(other => {
    if (other === id) return false;
    const shortcuts: readonly ShortcutToken[] = [
      SHORTCUTS[other], effectiveShortcut(config, other),
      ...(other === 'ZOOM_IN' ? ZOOM_IN_ALIASES : []),
    ];
    return shortcuts.some(shortcut => {
      const sameKey = shortcut.code && candidate.code
        ? shortcut.code === candidate.code
        : shortcut.key.toUpperCase() === candidate.key.toUpperCase();
      return sameKey && Boolean(shortcut.primary) === Boolean(candidate.primary)
        && Boolean(shortcut.alt) === Boolean(candidate.alt) && Boolean(shortcut.shift) === Boolean(candidate.shift);
    });
  });
}
