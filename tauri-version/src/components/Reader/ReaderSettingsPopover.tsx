import { t } from '../../i18n';
import type { ReaderFlow, ReaderSettings } from '../../modules/settings';
import { SegmentedControl } from '../Common/SegmentedControl';
import { Row } from '../Settings/Row';
import { NumberControl } from '../Settings/controls/NumberControl';

interface ReaderSettingsPopoverProps {
  settings: ReaderSettings;
  onChange: (partial: Partial<ReaderSettings>) => void;
  onClose: () => void;
}

export function ReaderSettingsPopover({
  settings,
  onChange,
  onClose,
}: ReaderSettingsPopoverProps) {
  return (
    <>
      <button
        type="button"
        className="q-reader-settings-scrim"
        aria-label={t('reader.closeSettings')}
        onClick={onClose}
      />
      <div className="q-reader-settings" role="dialog" aria-label={t('reader.settings')}>
        <Row label={t('reader.flow')}>
          <SegmentedControl
            stretch
            value={settings.flow}
            options={[
              { value: 'paginated', label: t('reader.flowPaginated') },
              { value: 'scrolled', label: t('reader.flowScrolled') },
            ]}
            onChange={(flow) => onChange({ flow: flow as ReaderFlow })}
          />
        </Row>
        <Row label={t('reader.fontSize')}>
          <NumberControl
            ariaLabel={t('reader.fontSize')}
            value={settings.fontSizeBase}
            min={12}
            max={32}
            onChange={(fontSizeBase) => onChange({ fontSizeBase })}
          />
        </Row>
        <Row label={t('reader.lineHeight')}>
          <NumberControl
            ariaLabel={t('reader.lineHeight')}
            value={settings.lineHeight}
            min={1.2}
            max={2.4}
            step={0.05}
            onChange={(lineHeight) => onChange({ lineHeight })}
          />
        </Row>
        <Row label={t('reader.columnWidth')}>
          <NumberControl
            ariaLabel={t('reader.columnWidthAria')}
            value={settings.maxWidthCh}
            min={30}
            max={120}
            onChange={(maxWidthCh) => onChange({ maxWidthCh })}
          />
        </Row>
      </div>
    </>
  );
}
