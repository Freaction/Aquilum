import type { AppConfig } from '../../../modules/settings';
import { Row } from '../Row';
import { t } from '../../../i18n';
import { Section } from '../Section';
import { NumberControl } from '../controls/NumberControl';
import type { SettingsSectionProps } from '../types';

export function SearchSection({ config, onChange }: SettingsSectionProps) {
  const patchSearch = (partial: Partial<AppConfig['search']>) => onChange({
    ...config,
    search: { ...config.search, ...partial },
  });

  return (
    <Section title={t('settings.search.section')}>
      <Row label={t('settings.search.candidatePool')}>
        <NumberControl
          ariaLabel={t('settings.search.candidatePool')}
          value={config.search.candidatePoolSize}
          min={32}
          max={2048}
          step={32}
          onChange={(candidatePoolSize) => patchSearch({ candidatePoolSize })}
        />
      </Row>
      <Row label={t('settings.search.maxQueryTerms')}>
        <NumberControl
          ariaLabel={t('settings.search.maxQueryTerms')}
          value={config.search.maxQueryTerms}
          min={8}
          max={128}
          onChange={(maxQueryTerms) => patchSearch({ maxQueryTerms })}
        />
      </Row>
    </Section>
  );
}
