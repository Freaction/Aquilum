import type { PluginSettingsProps } from '../registry';
import type { PluginSettings } from '../../modules/settings';
import { t } from '../../i18n';
import { Input } from '../../components/Common/Input';
import { Switch } from '../../components/Common/Switch';
import { ShortcutInput } from '../../components/Settings/controls/ShortcutInput';
import { Row } from '../../components/Settings/Row';

type Settings = PluginSettings['calendar'];

export function CalendarSettings({ config, onChange }: PluginSettingsProps) {
  const settings = config.plugins.calendar;
  const patch = (partial: Partial<Settings>) => onChange({
    ...config, plugins: { ...config.plugins, calendar: { ...settings, ...partial } },
  });
  const patchWeekly = (partial: Partial<Settings['weekly']>) => patch({ weekly: { ...settings.weekly, ...partial } });
  return <>
    <Row label={t('plugins.calendar.openTodayShortcut')}>
      <ShortcutInput config={config} id="OPEN_TODAY_NOTE" value={settings.openTodayShortcut} label={t('plugins.calendar.openTodayShortcut')} onChange={openTodayShortcut => patch({ openTodayShortcut })} />
    </Row>
    <Row label={t('plugins.calendar.showWeekNumbers')}>
      <Switch checked={settings.showWeekNumbers} label={t('plugins.calendar.showWeekNumbers')} onChange={showWeekNumbers => patch({ showWeekNumbers })} />
    </Row>
    <Row label={t('plugins.calendar.weeklyEnabled')}>
      <Switch checked={settings.weekly.enabled} label={t('plugins.calendar.weeklyEnabled')} onChange={enabled => patchWeekly({ enabled })} />
    </Row>
    <Row label={t('plugins.calendar.weeklyFolder')}>
      <Input value={settings.weekly.folder} ariaLabel={t('plugins.calendar.weeklyFolder')} onChange={folder => patchWeekly({ folder })} />
    </Row>
    <Row label={t('plugins.calendar.weeklyFormat')} description={t('plugins.calendar.weeklyFormatHint')}>
      <Input value={settings.weekly.format} ariaLabel={t('plugins.calendar.weeklyFormat')} onChange={format => patchWeekly({ format })} />
    </Row>
    <Row label={t('plugins.calendar.weeklyTemplate')}>
      <Input value={settings.weekly.template} ariaLabel={t('plugins.calendar.weeklyTemplate')} onChange={template => patchWeekly({ template })} />
    </Row>
  </>;
}
