import { t } from '../../i18n';
import type { PluginSettingsProps } from '../registry';
import { Row } from '../../components/Settings/Row';
import { Dropdown } from '../../components/Common/Dropdown';

export function CodeStylerSettings({ config, onChange }: PluginSettingsProps) {
  const settings = config.plugins.codeStyler;
  return <div className="q-code-styler-settings">
    {(['lineNumbers', 'copyButton', 'header'] as const).map(key => <Row key={key} label={t(`plugins.codeStyler.${key}`)}>
      <Dropdown ariaLabel={t(`plugins.codeStyler.${key}`)} value={String(settings[key])}
        options={[{ value: 'true', label: t('plugins.codeStyler.on') }, { value: 'false', label: t('plugins.codeStyler.off') }]}
        onChange={value => onChange({ ...config, plugins: { ...config.plugins, codeStyler: { ...settings, [key]: value === 'true' } } })} />
    </Row>)}
  </div>;
}
