import { historyRetentionOptions } from '../../../modules/history';
import { t } from '../../../i18n';
import { Dropdown } from '../../Common/Dropdown';
import { Row } from '../Row';
import { Section } from '../Section';
import type { SettingsSectionProps } from '../types';

export function HistorySection({ config, onChange }: SettingsSectionProps) {
  const retentionDays = config.history?.retentionDays ?? 0;

  return (
    <Section title={t('settings.history.section')}>
      <Row
        label={t('settings.history.retention')}
        description={t('settings.history.retentionHint')}
      >
        <Dropdown
          ariaLabel={t('settings.history.retention')}
          value={String(retentionDays)}
          options={historyRetentionOptions()}
          onChange={(value) => onChange({ ...config, history: { retentionDays: Number(value) } })}
        />
      </Row>
    </Section>
  );
}
