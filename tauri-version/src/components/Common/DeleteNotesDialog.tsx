import { isMarkdownPath } from '../../modules/documents/fileGateway';
import { fileStem } from '../../modules/paths';
import { ConfirmDialog } from './ConfirmDialog';
import { t } from '../../i18n';

interface DeleteNotesDialogProps {
  targets: readonly string[];
  pending: boolean;
  onCancel: () => void;
  onConfirm: () => void;
}

export function DeleteNotesDialog({
  targets,
  pending,
  onCancel,
  onConfirm,
}: DeleteNotesDialogProps) {
  const many = targets.length > 1;
  const folder = !many && targets.length === 1 && !isMarkdownPath(targets[0]);

  return (
    <ConfirmDialog
      open={targets.length > 0}
      title={many ? t('confirm.notesTitle') : folder ? t('confirm.folderTitle') : t('confirm.noteTitle')}
      description={many
        ? t('confirm.manyDescription', { count: targets.length })
        : folder
          ? t('confirm.folderDescription', { name: fileStem(targets[0]) })
          : t('confirm.noteDescription', { name: targets[0] ? fileStem(targets[0]) : '' })}
      confirmLabel={many ? t('confirm.deleteMany', { count: targets.length }) : t('common.delete')}
      pending={pending}
      onCancel={onCancel}
      onConfirm={onConfirm}
    />
  );
}
