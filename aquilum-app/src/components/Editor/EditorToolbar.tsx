import { t } from '../../i18n';
import { ArrowLeft, ArrowRight, MoreHorizontal, Search } from 'lucide';
import { Icon } from '../Common/Icon';
import { Button } from '../Common/Button';
import { IconButton } from '../Common/IconButton';
import { Menu, type MenuItem, type MenuPosition } from '../Common/Menu';
import { useCallback, useMemo, useRef, useState } from 'react';
import { useSettingsStore } from '../../modules/settings';
import { effectiveShortcut, formatShortcut } from '../../config/shortcuts';
import { toggleReadingMode, useReadingMode } from '../../plugins/editor/readingMode';
import './EditorToolbar.css';

interface EditorToolbarProps {
  fileName: string;
  canGoBack: boolean;
  canGoForward: boolean;
  onNavigate: (delta: -1 | 1) => void;
  onSearch: () => void;
  onExportPdf: () => void;
  focusMode: boolean;
  onToggleFocusMode: () => void;
}

export function EditorToolbar({ fileName, canGoBack, canGoForward, onNavigate, onSearch, onExportPdf, focusMode, onToggleFocusMode }: EditorToolbarProps) {
  const { config, updateConfig } = useSettingsStore();
  const readingMode = useReadingMode();
  const triggerRef = useRef<HTMLSpanElement>(null);
  const [position, setPosition] = useState<MenuPosition | null>(null);
  const close = useCallback(() => setPosition(null), []);
  const items: MenuItem[] = useMemo(() => [
    {
      id: 'toggle-editor-width',
      label: t(config?.editor.fullWidth ? 'editor.widthReadable' : 'editor.widthFull'),
      shortcut: formatShortcut(effectiveShortcut(config, 'TOGGLE_FULL_WIDTH')),
      disabled: !config,
      onSelect: () => { if (config) void updateConfig({ ...config, editor: { ...config.editor, fullWidth: !config.editor.fullWidth } }); },
    },
    ...(config?.plugins.readingMode.enabled ? [{
      id: 'reading-mode',
      label: t('plugins.readingMode.name'),
      shortcut: formatShortcut(effectiveShortcut(config, 'TOGGLE_READING_MODE')),
      checked: readingMode,
      onSelect: toggleReadingMode,
    }] : []),
    { id: 'export-pdf', label: t('editor.exportPdf'), onSelect: onExportPdf },
  ], [onExportPdf, config, updateConfig, readingMode]);
  const openMenu = () => {
    const bounds = triggerRef.current?.getBoundingClientRect();
    if (bounds) {
      setPosition({ top: bounds.bottom + 4, left: bounds.left });
    }
  };
  return <div className="q-editor-toolbar">
    <div className="q-editor-toolbar__side">
      {!focusMode && <>
        <IconButton label={t('editor.back')} size="medium" disabled={!canGoBack} onClick={() => onNavigate(-1)}><Icon icon={ArrowLeft} /></IconButton>
        <IconButton label={t('editor.forward')} size="medium" disabled={!canGoForward} onClick={() => onNavigate(1)}><Icon icon={ArrowRight} /></IconButton>
      </>}
    </div>
    {!focusMode && <div className="q-editor-toolbar__name" title={fileName}>{fileName}</div>}
    <div className="q-editor-toolbar__side">
      <Button variant="ghost" size="xs" aria-pressed={focusMode} onClick={onToggleFocusMode}>{t('editor.focusMode')}</Button>
      {!focusMode && <IconButton label={t('editor.searchInNote')} size="medium" onClick={onSearch}><Icon icon={Search} /></IconButton>}
      <span ref={triggerRef}><IconButton label={t('editor.noteActions')} size="medium" aria-expanded={Boolean(position)} onClick={openMenu}><Icon icon={MoreHorizontal} /></IconButton></span>
      <Menu open={Boolean(position)} position={position} items={items} showCheck onClose={close} ariaLabel={t('editor.noteActions')} excludeRef={triggerRef} />
    </div>
  </div>;
}
