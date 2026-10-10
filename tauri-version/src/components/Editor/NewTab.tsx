import { t } from '../../i18n';
import { TextButton } from '../Common/TextButton';
import './NewTab.css';

interface NewTabProps {
  onCreate: () => void | Promise<void>;
  onOpen: () => void;
  onClose: () => void;
  disabled?: boolean;
}

export function NewTab({
  onCreate,
  onOpen,
  onClose,
  disabled = false,
}: NewTabProps) {
  return (
    <div className="q-new-tab">
      <TextButton disabled={disabled} onClick={() => void onCreate()}>
        {t('editor.createNote')}
      </TextButton>
      <TextButton onClick={onOpen}>{t('editor.openFile')}</TextButton>
      <TextButton onClick={onClose}>{t('editor.close')}</TextButton>
    </div>
  );
}
