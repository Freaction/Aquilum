import { Dropdown } from '../Common/Dropdown';
import { t } from '../../i18n';
import type { FontSettings } from '../../modules/settings';
import { BODY_WEIGHTS, SCOPE_FAMILIES, type FontFamily, type FontScope } from '../../fonts/catalog';
import { Row } from './Row';
import { NumberControl } from './controls/NumberControl';

interface FontSettingsRowsProps {
  scope: FontScope;
  font: FontSettings;
  onChange: (partial: Partial<FontSettings>) => void;
}

const NAMED_WEIGHTS: Partial<Record<number, string>> = {
  400: 'settings.font.weightRegular',
  450: 'settings.font.weightText',
  500: 'settings.font.weightMedium',
  600: 'settings.font.weightSemibold',
};

function weightLabel(weight: number): string {
  const key = NAMED_WEIGHTS[weight];
  return key ? `${t(key)} · ${weight}` : String(weight);
}

export function FontSettingsRows({ scope, font, onChange }: FontSettingsRowsProps) {
  const familyOptions = SCOPE_FAMILIES[scope].map((family) => ({ value: family, label: family }));
  const weightOptions = BODY_WEIGHTS.map((weight) => ({ value: String(weight), label: weightLabel(weight) }));

  return (
    <>
      <Row label={t('settings.font.family')}>
        <Dropdown
          ariaLabel={t('settings.font.family')}
          value={font.fontFamily}
          options={familyOptions}
          onChange={(fontFamily) => onChange({ fontFamily: fontFamily as FontFamily })}
        />
      </Row>
      <Row label={t('settings.font.weight')} description={t('settings.font.weightHint')}>
        <Dropdown
          ariaLabel={t('settings.font.weight')}
          value={String(font.fontWeight)}
          options={weightOptions}
          onChange={(value) => onChange({ fontWeight: Number(value) })}
        />
      </Row>
      <Row label={t('settings.font.size')}>
        <NumberControl
          ariaLabel={t('settings.font.size')}
          value={font.fontSizeBase}
          min={12}
          max={32}
          onChange={(fontSizeBase) => onChange({ fontSizeBase })}
        />
      </Row>
    </>
  );
}
