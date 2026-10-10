import { Switch } from '../../components/Common/Switch';
import { Row } from '../../components/Settings/Row';
import { t } from '../../i18n';
import type { PluginSettingsProps } from '../registry';

export function FolderCountsSettings({ config, onChange }: PluginSettingsProps) {
  return <>{(['showAllFiles', 'hideZero'] as const).map(field => <Row key={field} label={t(`plugins.folderCounts.${field}`)}>
    <Switch label={t(`plugins.folderCounts.${field}`)} checked={config.plugins.folderCounts[field]} onChange={value => onChange({ ...config, plugins: { ...config.plugins, folderCounts: { ...config.plugins.folderCounts, [field]: value } } })} />
  </Row>)}</>;
}
