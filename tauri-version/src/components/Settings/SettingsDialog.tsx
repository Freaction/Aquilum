import { useLocalState } from '../../modules/workspace/uiPersist';
import { Dialog } from '../Common/Dialog';
import { t } from '../../i18n';
import { ScrollArea } from '../Common/ScrollArea';
import { SettingsForm } from './SettingsForm';
import type { SettingsSectionId } from './types';
import { SettingsNav } from './SettingsNav';
import { settingsSections } from './constants';
import { useSettingsPersist } from './useSettingsPersist';
import './SettingsDialog.css';

interface SettingsDialogProps {
  open: boolean;
  workspacePath: string | null;
  homePage: string;
  onHomePageChange: (value: string) => void;
  onClose: () => void;
}

export function SettingsDialog({
  open,
  workspacePath,
  homePage,
  onHomePageChange,
  onClose,
}: SettingsDialogProps) {
  const { config, persist } = useSettingsPersist();
  const [storedSection, setSection] = useLocalState<SettingsSectionId>('aquilum_settings_section', 'ui');
  const section = settingsSections().some(({ id }) => id === storedSection) ? storedSection : 'ui';

  return (
    <Dialog
      open={open}
      title={t('settings.title')}
      closeLabel={t('settings.close')}
      className="q-settings-dialog"
      onClose={onClose}
    >
      <div className="q-settings-body">
        <SettingsNav section={section} onSelect={setSection} />

        <ScrollArea className="q-settings-content">
          {section !== 'shortcuts' && !config ? (
            <div className="q-settings-loading">{t('settings.loading')}</div>
          ) : (
            <SettingsForm
              config={config}
              section={section}
              workspacePath={workspacePath}
              homePage={homePage}
              onHomePageChange={onHomePageChange}
              onChange={persist}
            />
          )}
        </ScrollArea>
      </div>
    </Dialog>
  );
}
