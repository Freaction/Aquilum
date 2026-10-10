import type { SettingsSectionProps } from '../types';
import { DEFAULT_DAILY_NOTES_SETTINGS, type DailyNotesSettings } from '../../../modules/dailyNotes';
import { t } from '../../../i18n';
import { Input } from '../../Common/Input';
import { Switch } from '../../Common/Switch';
import { SegmentedControl } from '../../Common/SegmentedControl';
import { Row } from '../Row';
import { Section } from '../Section';
import { NumberControl } from '../controls/NumberControl';
import { DailyNotesFolderPicker } from '../controls/DailyNotesFolderPicker';

export function DailyNotesSection({ config, onChange, workspacePath = null }: SettingsSectionProps & { workspacePath?: string | null }) {
  const settings = { ...DEFAULT_DAILY_NOTES_SETTINGS, ...config.dailyNotes };
  const patch = (partial: Partial<DailyNotesSettings>) => onChange({
    ...config, dailyNotes: { ...settings, ...partial },
  });

  return <Section title={t('settings.dailyNotes.section')}>
    <Row label={t('settings.dailyNotes.folder')} description={t('settings.dailyNotes.folderHint')}>
      <DailyNotesFolderPicker workspacePath={workspacePath} value={settings.folder} onChange={folder => patch({ folder })} />
    </Row>
    <Row label={t('settings.dailyNotes.format')} description={t('settings.dailyNotes.formatHint')}>
      <Input value={settings.format} ariaLabel={t('settings.dailyNotes.format')} onChange={format => patch({ format })} />
    </Row>
    <Row label={t('settings.dailyNotes.template')} description={t('settings.dailyNotes.templateHint')}>
      <Input value={settings.template} ariaLabel={t('settings.dailyNotes.template')} onChange={template => patch({ template })} />
    </Row>
    <Row label={t('settings.dailyNotes.openOnStartup')}>
      <Switch checked={settings.openOnStartup} label={t('settings.dailyNotes.openOnStartup')} onChange={openOnStartup => patch({ openOnStartup })} />
    </Row>
    <Row label={t('settings.dailyNotes.wordsPerDot')} description={t('settings.dailyNotes.wordsPerDotHint')}>
      <NumberControl value={settings.wordsPerDot} ariaLabel={t('settings.dailyNotes.wordsPerDot')} min={0} max={10000} step={50} onChange={wordsPerDot => patch({ wordsPerDot: Math.round(wordsPerDot) })} />
    </Row>
    <Row label={t('settings.dailyNotes.weekStart')}>
      <SegmentedControl
        value={settings.weekStart}
        options={[
          { value: 'locale', label: t('settings.dailyNotes.locale') },
          { value: 'monday', label: t('settings.dailyNotes.monday') },
          { value: 'sunday', label: t('settings.dailyNotes.sunday') },
        ]}
        onChange={weekStart => patch({ weekStart: weekStart as DailyNotesSettings['weekStart'] })}
      />
    </Row>
    <Row label={t('settings.dailyNotes.confirmBeforeCreate')}>
      <Switch checked={settings.confirmBeforeCreate} label={t('settings.dailyNotes.confirmBeforeCreate')} onChange={confirmBeforeCreate => patch({ confirmBeforeCreate })} />
    </Row>
  </Section>;
}
