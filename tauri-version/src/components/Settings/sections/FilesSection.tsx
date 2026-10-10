import type { AppConfig } from '../../../modules/settings';
import { Row } from '../Row';
import { t } from '../../../i18n';
import { Section } from '../Section';
import { Input } from '../../Common/Input';

export function FilesSection({ config, onChange }: {
  config: AppConfig;
  onChange: (value: AppConfig) => void;
}) {
  return <Section title={t('settings.files.section')}>
    <Row label={t('settings.files.folder')} description={t('settings.files.folderHint')}>
      <Input value={config.files.folder} ariaLabel={t('settings.files.folder')}
        onChange={(folder) => onChange({ ...config, files: { folder } })} />
    </Row>
  </Section>;
}
