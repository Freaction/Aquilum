import type { AppConfig } from '../../../modules/settings';
import { Switch } from '../../Common/Switch';
import { t } from '../../../i18n';
import { Row } from '../Row';
import { Section } from '../Section';
import { NumberControl } from '../controls/NumberControl';
import type { SettingsSectionProps } from '../types';

export function AnalysisSection({ config, onChange }: SettingsSectionProps) {
  const patchAnalysis = (partial: Partial<AppConfig['analysis']>) => onChange({
    ...config,
    analysis: { ...config.analysis, ...partial },
  });

  const patchBm25f = (partial: Partial<AppConfig['analysis']['bm25fParams']>) => onChange({
    ...config,
    analysis: {
      ...config.analysis,
      bm25fParams: { ...config.analysis.bm25fParams, ...partial },
    },
  });

  return (
    <>
      <Section title={t('settings.analysis.methods')}>
        <Row label={t('settings.analysis.bm25f')}>
          <Switch
            label="BM25F"
            checked={config.analysis.enableBm25f}
            onChange={(enableBm25f) => patchAnalysis({ enableBm25f })}
          />
        </Row>
        <Row label={t('settings.analysis.adamicAdar')}>
          <Switch
            label="Adamic-Adar"
            checked={config.analysis.enableAdamicAdar}
            onChange={(enableAdamicAdar) => patchAnalysis({ enableAdamicAdar })}
          />
        </Row>
        <Row label={t('settings.analysis.wiki')}>
          <Switch
            label="Wiki"
            checked={config.analysis.enableWikixiv ?? true}
            onChange={(enableWikixiv) => patchAnalysis({ enableWikixiv })}
          />
        </Row>
      </Section>
      {config.analysis.enableBm25f && (
        <Section title={t('settings.analysis.bm25fParams')}>
          <Row label="k1">
            <NumberControl
              ariaLabel="k1"
              value={config.analysis.bm25fParams.k1}
              min={0.1}
              max={3}
              step={0.1}
              onChange={(k1) => patchBm25f({ k1 })}
            />
          </Row>
          <Row label="k3">
            <NumberControl
              ariaLabel="k3"
              value={config.analysis.bm25fParams.k3}
              min={1}
              max={20}
              step={0.5}
              onChange={(k3) => patchBm25f({ k3 })}
            />
          </Row>
          <Row label={t('settings.analysis.bTitle')}>
            <NumberControl
              ariaLabel={t('settings.analysis.bTitle')}
              value={config.analysis.bm25fParams.bTitle}
              min={0}
              max={1}
              step={0.05}
              onChange={(bTitle) => patchBm25f({ bTitle })}
            />
          </Row>
          <Row label={t('settings.analysis.bBody')}>
            <NumberControl
              ariaLabel={t('settings.analysis.bBody')}
              value={config.analysis.bm25fParams.bBody}
              min={0}
              max={1}
              step={0.05}
              onChange={(bBody) => patchBm25f({ bBody })}
            />
          </Row>
          <Row label={t('settings.analysis.titleWeight')}>
            <NumberControl
              ariaLabel={t('settings.analysis.titleWeight')}
              value={config.analysis.bm25fParams.titleWeight}
              min={0.5}
              max={10}
              step={0.1}
              onChange={(titleWeight) => patchBm25f({ titleWeight })}
            />
          </Row>
        </Section>
      )}
    </>
  );
}
