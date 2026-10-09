import { t } from '../i18n';
import type { AppConfig } from '../modules/settings';
import { effectiveShortcut, SHORTCUTS, type ShortcutToken } from './shortcuts';

interface ShortcutCatalogEntry {
  label: string;
  shortcut: ShortcutToken;
}

interface ShortcutCatalogGroup {
  title: string;
  items: ShortcutCatalogEntry[];
}

export function shortcutCatalog(config?: AppConfig | null): ShortcutCatalogGroup[] {
  return [
    {
      title: t('settings.shortcuts.global'),
      items: [
        { label: t('settings.shortcuts.newTab'), shortcut: SHORTCUTS.NEW_FILE },
        { label: t('settings.shortcuts.newFromTemplate'), shortcut: SHORTCUTS.NEW_FROM_TEMPLATE },
        { label: t('settings.shortcuts.globalSearch'), shortcut: SHORTCUTS.GLOBAL_SEARCH },
        { label: t('settings.shortcuts.focusMode'), shortcut: SHORTCUTS.FOCUS_MODE },
        { label: t('settings.shortcuts.pageSearch'), shortcut: SHORTCUTS.PAGE_SEARCH },
        { label: t('settings.shortcuts.zoomIn'), shortcut: SHORTCUTS.ZOOM_IN },
        { label: t('settings.shortcuts.zoomOut'), shortcut: SHORTCUTS.ZOOM_OUT },
        { label: t('settings.shortcuts.zoomReset'), shortcut: SHORTCUTS.ZOOM_RESET },
        { label: t('shortcuts.openTodayNote'), shortcut: effectiveShortcut(config, 'OPEN_TODAY_NOTE') },
        { label: t('shortcuts.toggleFullWidth'), shortcut: effectiveShortcut(config, 'TOGGLE_FULL_WIDTH') },
        { label: t('shortcuts.toggleReadingMode'), shortcut: effectiveShortcut(config, 'TOGGLE_READING_MODE') },
      ],
    },
    {
      title: t('settings.shortcuts.editor'),
      items: [
        { label: t('settings.shortcuts.bold'), shortcut: SHORTCUTS.BOLD },
        { label: t('settings.shortcuts.italic'), shortcut: SHORTCUTS.ITALIC },
        { label: t('settings.shortcuts.strikethrough'), shortcut: SHORTCUTS.STRIKETHROUGH },
        { label: t('settings.shortcuts.indent'), shortcut: SHORTCUTS.INDENT },
        { label: t('settings.shortcuts.outdent'), shortcut: SHORTCUTS.OUTDENT },
        { label: t('settings.shortcuts.lineBreak'), shortcut: SHORTCUTS.LINE_BREAK },
      ],
    },
    {
      title: t('settings.shortcuts.searchWindow'),
      items: [
        { label: t('settings.shortcuts.searchNext'), shortcut: SHORTCUTS.SEARCH_NEXT },
        { label: t('settings.shortcuts.searchPrevious'), shortcut: SHORTCUTS.SEARCH_PREVIOUS },
        { label: t('settings.shortcuts.searchOpen'), shortcut: SHORTCUTS.SEARCH_OPEN },
        { label: t('settings.shortcuts.searchOpenNewPane'), shortcut: SHORTCUTS.SEARCH_OPEN_NEW_PANE },
        { label: t('settings.shortcuts.searchCreate'), shortcut: SHORTCUTS.SEARCH_CREATE },
      ],
    },
    {
      title: t('settings.shortcuts.pageSearchWindow'),
      items: [
        { label: t('settings.shortcuts.searchNext'), shortcut: SHORTCUTS.PAGE_SEARCH_NEXT },
        { label: t('settings.shortcuts.searchPrevious'), shortcut: SHORTCUTS.PAGE_SEARCH_PREVIOUS },
      ],
    },
    {
      title: t('settings.shortcuts.reader'),
      items: [
        { label: t('settings.shortcuts.nextPage'), shortcut: SHORTCUTS.READER_NEXT_PAGE },
        { label: t('settings.shortcuts.previousPage'), shortcut: SHORTCUTS.READER_PREVIOUS_PAGE },
      ],
    },
  ];
}
