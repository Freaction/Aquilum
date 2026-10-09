import { Plug } from 'lucide';
import { t } from '../../../i18n';
import { PLUGINS, pluginSettings } from '../../../plugins/registry';
import { Icon } from '../../Common/Icon';
import { Button } from '../../Common/Button';
import { Switch } from '../../Common/Switch';
import { Row } from '../Row';
import { Section } from '../Section';
import type { SettingsSectionId, SettingsSectionProps } from '../types';
import './PluginsSection.css';
import { ObsidianImport } from '../../../plugins/import/ObsidianImport';

interface PluginsSectionProps extends SettingsSectionProps {
  workspacePath?: string | null;
  onNavigate: (section: SettingsSectionId) => void;
}

export function PluginsSection({ config, onChange, workspacePath = null, onNavigate }: PluginsSectionProps) {
  return <div className="q-plugins">
    <h2 className="q-plugins__title">{t('plugins.title')}</h2>
    {PLUGINS.map(({ id, icon, settings: Settings }) => <Section key={id} title={t(`plugins.${id}.name`)}>
      <div className="q-plugins__row">
        <Icon icon={icon} />
        <Row label={t('plugins.enabled')} description={t(`plugins.${id}.description`)}>
          <Switch checked={pluginSettings(config, id).enabled} label={t(`plugins.${id}.name`)} onChange={enabled => onChange({
            ...config, plugins: { ...config.plugins, [id]: { ...pluginSettings(config, id), enabled } },
          })} />
        </Row>
      </div>
      {id === 'calendar' && <div className="q-plugins__actions">
        <Button variant="ghost" size="s" onClick={() => onNavigate('dailyNotes')}>{t('plugins.calendar.dailyNotes')}</Button>
      </div>}
      {Settings && <details className="q-plugins__settings">
        <summary>{t('plugins.settings')}</summary>
        <Settings config={config} onChange={onChange} workspacePath={workspacePath} />
      </details>}
    </Section>)}
    <Section title={t('plugins.builtinMcp.name')}>
      <div className="q-plugins__row">
        <Icon icon={Plug} />
        <Row label={t('plugins.builtinMcp.name')} description={t('plugins.builtinMcp.description')} />
      </div>
      <div className="q-plugins__actions">
        <Button variant="ghost" size="s" onClick={() => onNavigate('mcp')}>{t('plugins.builtinMcp.settings')}</Button>
      </div>
    </Section>
    <div className="q-plugins__import"><ObsidianImport workspacePath={workspacePath} /></div>
  </div>;
}
