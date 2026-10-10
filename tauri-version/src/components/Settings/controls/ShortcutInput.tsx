import { useEffect, useState } from 'react';
import { Button } from '../../Common/Button';
import { t } from '../../../i18n';
import type { AppConfig } from '../../../modules/settings';
import { effectiveShortcut, formatShortcut, shortcutConflict, type ConfigurableShortcut, type ShortcutToken } from '../../../config/shortcuts';

interface ShortcutInputProps {
  config: AppConfig;
  id: ConfigurableShortcut;
  value: ShortcutToken | null;
  label: string;
  onChange: (shortcut: ShortcutToken | null) => void;
}

export function ShortcutInput({ config, id, value, label, onChange }: ShortcutInputProps) {
  const [recording, setRecording] = useState(false);
  const [error, setError] = useState(false);
  useEffect(() => {
    if (!recording) return;
    const capture = (event: KeyboardEvent) => {
      event.preventDefault();
      event.stopImmediatePropagation();
      if (event.key === 'Escape') { setRecording(false); setError(false); return; }
      if ((!event.ctrlKey && !event.metaKey && !event.altKey) || !event.code
        || ['Control', 'Meta', 'Alt', 'Shift', 'AltGraph', 'Unidentified'].includes(event.key)) return;
      const key = /^(Key[A-Z]|Digit[0-9])$/.test(event.code) ? event.code.replace(/^(Key|Digit)/, '') : event.key;
      const shortcut = { code: event.code, key, primary: event.ctrlKey || event.metaKey, alt: event.altKey, shift: event.shiftKey };
      if (shortcutConflict(config, id, shortcut)) { setError(true); return; }
      onChange(shortcut);
      setError(false);
      setRecording(false);
    };
    window.addEventListener('keydown', capture, true);
    return () => window.removeEventListener('keydown', capture, true);
  }, [recording, config, id, onChange]);

  return <div role="group" aria-label={label}>
    <Button variant="ghost" size="s" aria-label={label} aria-pressed={recording}
      onClick={() => { setRecording(true); setError(false); }} onBlur={() => setRecording(false)}>
      <span aria-live="polite">{recording ? t('settings.shortcuts.recording') : formatShortcut(value ?? effectiveShortcut(null, id))}</span>
    </Button>
    <Button variant="ghost" size="s" onClick={() => {
      setRecording(false);
      if (shortcutConflict(config, id, effectiveShortcut(null, id))) { setError(true); return; }
      setError(false);
      onChange(null);
    }}>{t('settings.shortcuts.reset')}</Button>
    {error && <span role="alert">{t('settings.shortcuts.conflict')}</span>}
  </div>;
}
