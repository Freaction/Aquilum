import type { PluginSettingsProps } from '../registry';
import type { PluginSettings } from '../../modules/settings';
import { t } from '../../i18n';
import { Input } from '../../components/Common/Input';
import { Switch } from '../../components/Common/Switch';
import { SegmentedControl } from '../../components/Common/SegmentedControl';
import { NumberControl } from '../../components/Settings/controls/NumberControl';
import { Row } from '../../components/Settings/Row';

export function GitSettings({ config, onChange }: PluginSettingsProps) {
  const settings = config.plugins.gitSync;
  const patch = (partial: Partial<PluginSettings['gitSync']>) => onChange({
    ...config, plugins: { ...config.plugins, gitSync: { ...settings, ...partial } },
  });
  return <>
    <Row label={t('plugins.gitSync.commitMessage')}>
      <Input value={settings.commitMessage} ariaLabel={t('plugins.gitSync.commitMessage')} onChange={commitMessage => patch({ commitMessage })} />
    </Row>
    <Row label={t('plugins.gitSync.commitDateFormat')}>
      <Input value={settings.commitDateFormat} ariaLabel={t('plugins.gitSync.commitDateFormat')} onChange={commitDateFormat => patch({ commitDateFormat })} />
    </Row>
    <Row label={t('plugins.gitSync.autoBackupMinutes')} description={t('plugins.gitSync.autoBackupHint')}>
      <NumberControl value={settings.autoBackupMinutes} min={0} max={1440} ariaLabel={t('plugins.gitSync.autoBackupMinutes')} onChange={autoBackupMinutes => patch({ autoBackupMinutes: Math.round(autoBackupMinutes) })} />
    </Row>
    <Row label={t('plugins.gitSync.pullOnOpen')}>
      <Switch checked={settings.pullOnOpen} label={t('plugins.gitSync.pullOnOpen')} onChange={pullOnOpen => patch({ pullOnOpen })} />
    </Row>
    <Row label={t('plugins.gitSync.push')}>
      <Switch checked={settings.push} label={t('plugins.gitSync.push')} onChange={push => patch({ push })} />
    </Row>
    <Row label={t('plugins.gitSync.syncMethod')}>
      <SegmentedControl value={settings.syncMethod} options={[
        { value: 'merge', label: t('plugins.gitSync.merge') }, { value: 'rebase', label: t('plugins.gitSync.rebase') },
      ]} onChange={syncMethod => patch({ syncMethod: syncMethod as 'merge' | 'rebase' })} />
    </Row>
  </>;
}
