import type { PluginSettingsProps } from '../registry';
import { Row } from '../../components/Settings/Row';
import { ShortcutInput } from '../../components/Settings/controls/ShortcutInput';
import { t } from '../../i18n';

export function ReadingModeSettings({ config, onChange }: PluginSettingsProps) {
  return <Row label={t('plugins.readingMode.shortcut')}>
    <ShortcutInput config={config} id="TOGGLE_READING_MODE" value={config.plugins.readingMode.shortcut}
      label={t('plugins.readingMode.shortcut')} onChange={shortcut => onChange({
        ...config, plugins: { ...config.plugins, readingMode: { ...config.plugins.readingMode, shortcut } },
      })} />
  </Row>;
}
