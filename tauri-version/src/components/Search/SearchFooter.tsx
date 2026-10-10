import { t } from '../../i18n';
import { ArrowUpDown, CornerDownLeft } from 'lucide';
import { Icon } from '../Common/Icon';
import type { ReactNode } from 'react';
import { shortcutModifiers, SHORTCUTS, type ShortcutToken } from '../../config/shortcuts';
import { DialogFooter } from '../Common/Dialog';

export function SearchFooter() {
  return (
    <DialogFooter align="center">
      <KeyHint shortcut={SHORTCUTS.SEARCH_NEXT} label={t('search.navigate')}>
        <Icon icon={ArrowUpDown} />
      </KeyHint>
      <KeyHint shortcut={SHORTCUTS.SEARCH_OPEN} label={t('search.open')}>
        <Icon icon={CornerDownLeft} />
      </KeyHint>
      <KeyHint shortcut={SHORTCUTS.SEARCH_OPEN_NEW_PANE} label={t('search.newPane')}>
        <Icon icon={CornerDownLeft} />
      </KeyHint>
      <KeyHint shortcut={SHORTCUTS.SEARCH_CREATE} label={t('search.create')}>
        <Icon icon={CornerDownLeft} />
      </KeyHint>
    </DialogFooter>
  );
}

export function KeyHint({
  shortcut,
  label,
  children,
}: {
  shortcut: ShortcutToken;
  label: string;
  children?: ReactNode;
}) {
  return (
    <span className="q-search-key-hint">
      <kbd>
        {shortcutModifiers(shortcut).map((modifier) => <span key={modifier}>{modifier}</span>)}
        {children ?? <span>{shortcut.display ?? shortcut.key}</span>}
      </kbd>
      <span>{label}</span>
    </span>
  );
}
