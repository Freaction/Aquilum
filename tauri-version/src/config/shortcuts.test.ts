// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from 'vitest';
import { codeMirrorKey, formatShortcut, matchesShortcut, SHORTCUTS, effectiveShortcut, shortcutConflict } from './shortcuts';
import { legacyConfig } from '../plugins/testFixtures';
import { DEFAULT_PLUGIN_SETTINGS } from '../modules/settings';
afterEach(() => vi.restoreAllMocks());

describe('global shortcut tokens', () => {
  it('matches physical keys independently of the active keyboard layout', () => {
    const russianLayoutEvent = {
      code: 'KeyO',
      key: 'щ',
      ctrlKey: true,
      metaKey: false,
      altKey: false,
      shiftKey: false,
    } as KeyboardEvent;

    expect(matchesShortcut(russianLayoutEvent, SHORTCUTS.GLOBAL_SEARCH)).toBe(true);
  });

  it('does not consume modified variants of a shortcut', () => {
    const shiftedEvent = {
      code: 'KeyO',
      key: 'O',
      ctrlKey: true,
      metaKey: false,
      altKey: false,
      shiftKey: true,
    } as KeyboardEvent;

    expect(matchesShortcut(shiftedEvent, SHORTCUTS.GLOBAL_SEARCH)).toBe(false);
  });

  it('matches search command variants', () => {
    const event = (key: string, modifiers: Partial<KeyboardEvent> = {}) => ({
      key,
      ctrlKey: false,
      metaKey: false,
      altKey: false,
      shiftKey: false,
      ...modifiers,
    } as KeyboardEvent);

    expect(matchesShortcut(event('ArrowUp'), SHORTCUTS.SEARCH_PREVIOUS)).toBe(true);
    expect(matchesShortcut(event('Enter'), SHORTCUTS.SEARCH_OPEN)).toBe(true);
    expect(matchesShortcut(event('Enter', { ctrlKey: true }), SHORTCUTS.SEARCH_OPEN_NEW_PANE)).toBe(true);
    expect(matchesShortcut(event('Enter', { shiftKey: true }), SHORTCUTS.SEARCH_CREATE)).toBe(true);
    expect(matchesShortcut(event('Enter', { ctrlKey: true, shiftKey: true }), SHORTCUTS.SEARCH_CREATE)).toBe(false);
  });

  it('spells editor shortcuts the way CodeMirror keymaps expect', () => {
    expect(codeMirrorKey(SHORTCUTS.BOLD)).toBe('Mod-b');
    expect(codeMirrorKey(SHORTCUTS.STRIKETHROUGH)).toBe('Mod-Shift-s');
    expect(codeMirrorKey(SHORTCUTS.OUTDENT)).toBe('Shift-Tab');
    expect(codeMirrorKey(SHORTCUTS.LINE_BREAK)).toBe('Shift-Enter');
  });

  it('keeps focus mode apart from the page search', () => {
    const event = {
      code: 'KeyF',
      key: 'F',
      ctrlKey: true,
      metaKey: false,
      altKey: false,
      shiftKey: true,
    } as KeyboardEvent;

    expect(matchesShortcut(event, SHORTCUTS.FOCUS_MODE)).toBe(true);
    expect(matchesShortcut(event, SHORTCUTS.PAGE_SEARCH)).toBe(false);
  });
});
it('formats macOS symbols without separators and preserves other platforms', () => {
  vi.spyOn(navigator, 'platform', 'get').mockReturnValue('MacIntel');
  expect(formatShortcut(SHORTCUTS.TOGGLE_FULL_WIDTH)).toBe('⌘⌥W');
  expect(formatShortcut(SHORTCUTS.TOGGLE_READING_MODE)).toBe('⌘⇧E');
  vi.spyOn(navigator, 'platform', 'get').mockReturnValue('Win32');
  vi.spyOn(navigator, 'userAgent', 'get').mockReturnValue('Windows');
  expect(formatShortcut(SHORTCUTS.TOGGLE_FULL_WIDTH)).toBe('Ctrl + Alt + W');
});
it('resolves overrides and detects physical conflicts with effective and fixed shortcuts', () => {
  const config = { ...legacyConfig(), plugins: structuredClone(DEFAULT_PLUGIN_SETTINGS) };
  const custom = { code: 'KeyJ', key: 'J', primary: true, alt: true, shift: false };
  config.plugins.calendar.openTodayShortcut = custom;
  expect(effectiveShortcut(config, 'OPEN_TODAY_NOTE')).toEqual(custom);
  expect(effectiveShortcut(config, 'TOGGLE_FULL_WIDTH')).toEqual(SHORTCUTS.TOGGLE_FULL_WIDTH);
  expect(effectiveShortcut(null, 'OPEN_TODAY_NOTE')).toEqual(SHORTCUTS.OPEN_TODAY_NOTE);
  expect(shortcutConflict(config, 'TOGGLE_FULL_WIDTH', custom)).toBe('OPEN_TODAY_NOTE');
  expect(shortcutConflict(config, 'TOGGLE_FULL_WIDTH', SHORTCUTS.OPEN_TODAY_NOTE)).toBe('OPEN_TODAY_NOTE');
  expect(shortcutConflict(config, 'TOGGLE_FULL_WIDTH', { code: 'Equal', key: '+', primary: true, shift: true })).toBe('ZOOM_IN');
  expect(shortcutConflict(config, 'TOGGLE_FULL_WIDTH', { code: 'KeyB', key: 'B', primary: true })).toBe('BOLD');
  expect(shortcutConflict(config, 'TOGGLE_FULL_WIDTH', SHORTCUTS.TOGGLE_FULL_WIDTH)).toBeUndefined();
});
