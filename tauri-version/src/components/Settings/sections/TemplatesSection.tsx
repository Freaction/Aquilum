import { useEffect, useState } from 'react';
import type { AppConfig } from '../../../modules/settings';
import { createStarterTemplates, templatesDirectoryExists, templatesFolderName } from '../../../modules/templates';
import { Row } from '../Row';
import { t } from '../../../i18n';
import { Section } from '../Section';
import { TextButton } from '../../Common/TextButton';
import { Input } from '../../Common/Input';

export function TemplatesSection({ config, workspacePath, onChange }: {
  config: AppConfig;
  workspacePath: string | null;
  onChange: (value: AppConfig) => void;
}) {
  const [directoryExists, setDirectoryExists] = useState<boolean | null>(null);
  useEffect(() => {
    let active = true;
    setDirectoryExists(null);
    if (!workspacePath) return () => { active = false; };
    void templatesDirectoryExists(workspacePath, config.templates.folder)
      .then((exists) => { if (active) setDirectoryExists(exists); });
    return () => { active = false; };
  }, [workspacePath, config.templates.folder]);
  const createTemplates = () => {
    if (!workspacePath) return;
    createStarterTemplates(workspacePath, config.templates.folder)
      .then(() => setDirectoryExists(true))
      .catch((error) => console.error('Failed to create starter templates', error));
  };
  return <Section title={t('settings.templates.section')}>
    <Row label={t('settings.templates.folder')} description={t('settings.templates.folderHint')}><Input value={config.templates.folder}
      ariaLabel={t('settings.templates.folder')}
      onChange={(folder) => onChange({ ...config, templates: { folder } })} /></Row>
    {directoryExists === false && <Row label={t('settings.templates.create')} description={t('settings.templates.createHint', { folder: templatesFolderName(config.templates.folder) })}>
      <TextButton onClick={createTemplates}>{t('settings.templates.createButton')}</TextButton>
    </Row>}
  </Section>;
}
