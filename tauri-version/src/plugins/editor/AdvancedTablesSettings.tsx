import { t } from '../../i18n';
import type { PluginSettingsProps } from '../registry';
import { Row } from '../../components/Settings/Row';
import { Dropdown } from '../../components/Common/Dropdown';

export function AdvancedTablesSettings({ config, onChange }: PluginSettingsProps) {
  const settings = config.plugins.advancedTables;
  return <Row label={t('plugins.advancedTables.formatOnLeave')}>
    <Dropdown ariaLabel={t('plugins.advancedTables.formatOnLeave')} value={String(settings.formatOnLeave)}
      options={[{ value: 'true', label: t('plugins.advancedTables.on') }, { value: 'false', label: t('plugins.advancedTables.off') }]}
      onChange={value => onChange({ ...config, plugins: { ...config.plugins, advancedTables: { ...settings, formatOnLeave: value === 'true' } } })} />
  </Row>;
}
