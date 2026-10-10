import type { AppConfig, ReaderFlow } from '../../../modules/settings';
import { SegmentedControl } from '../../Common/SegmentedControl';
import { t } from '../../../i18n';
import { Switch } from '../../Common/Switch';
import { FontSettingsRows } from '../FontSettingsRows';
import { Row } from '../Row';
import { Section } from '../Section';
import { NumberControl } from '../controls/NumberControl';
import type { SettingsSectionProps } from '../types';

export function ReaderSection({ config, onChange }: SettingsSectionProps) {
  const patchReader = (partial: Partial<AppConfig['reader']>) => onChange({
    ...config,
    reader: { ...config.reader, ...partial },
  });

  return (
    <>
      <Section title={t('settings.font.section')}>
        <FontSettingsRows
          scope="reader"
          font={config.reader}
          onChange={patchReader}
        />
        <Row label={t('settings.reader.lineHeight')}>
          <NumberControl
            ariaLabel={t('settings.reader.lineHeight')}
            value={config.reader.lineHeight}
            min={1.2}
            max={2.4}
            step={0.05}
            onChange={(lineHeight) => patchReader({ lineHeight })}
          />
        </Row>
      </Section>
      <Section title={t('settings.reader.page')}>
        <Row
          label={t('settings.reader.flow')}
          description={t('settings.reader.flowHint')}
        >
          <SegmentedControl
            value={config.reader.flow}
            options={[
              { value: 'paginated', label: t('settings.reader.flowPaginated') },
              { value: 'scrolled', label: t('settings.reader.flowScrolled') },
            ]}
            onChange={(flow) => patchReader({ flow: flow as ReaderFlow })}
          />
        </Row>
        <Row
          label={t('settings.reader.maxWidth')}
          description={t('settings.reader.maxWidthHint')}
        >
          <NumberControl
            ariaLabel={t('settings.reader.maxWidthAria')}
            value={config.reader.maxWidthCh}
            min={30}
            max={120}
            onChange={(maxWidthCh) => patchReader({ maxWidthCh })}
          />
        </Row>
        <Row label={t('settings.reader.margin')} description={t('settings.reader.marginHint')}>
          <NumberControl
            ariaLabel={t('settings.reader.marginAria')}
            value={config.reader.marginPx}
            min={0}
            max={160}
            step={4}
            onChange={(marginPx) => patchReader({ marginPx })}
          />
        </Row>
      </Section>
      <Section title={t('settings.reader.typesetting')}>
        <Row label={t('settings.reader.justify')} description={t('settings.reader.justifyHint')}>
          <Switch
            checked={config.reader.justify}
            label={t('settings.reader.justify')}
            onChange={(justify) => patchReader({ justify })}
          />
        </Row>
        <Row label={t('settings.reader.hyphenate')} description={t('settings.reader.hyphenateHint')}>
          <Switch
            checked={config.reader.hyphenate}
            label={t('settings.reader.hyphenate')}
            onChange={(hyphenate) => patchReader({ hyphenate })}
          />
        </Row>
      </Section>
    </>
  );
}
