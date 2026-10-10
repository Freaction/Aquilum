import { Switch } from '../../components/Common/Switch';
import { Row } from '../../components/Settings/Row';
import { t } from '../../i18n';
import type { PluginSettingsProps } from '../registry';

export function FileColorsSettings({ config, onChange }: PluginSettingsProps) {
  return <>{(['cascade', 'background'] as const).map(field => <Row key={field} label={t(`plugins.fileColors.${field}`)}>
    <Switch label={t(`plugins.fileColors.${field}`)} checked={config.plugins.fileColors[field]} onChange={value => onChange({ ...config, plugins: { ...config.plugins, fileColors: { ...config.plugins.fileColors, [field]: value } } })} />
  </Row>)}</>;
}
