import { useCallback, useEffect, useState } from 'react';
import { formatDateTime, t } from '../../../i18n';
import { fileName, fileStem, parentDirectory } from '../../../modules/paths';
import { formatTrashCount, listTrash, restoreDeletion, type TrashedDeletion, type TrashPage } from '../../../modules/trash';
import { TextButton } from '../../Common/TextButton';
import { Row } from '../Row';
import { Section } from '../Section';

const PAGE_SIZE = 20;

function isFolder(deletion: TrashedDeletion) {
  return deletion.files > 1;
}

function deletionDescription(deletion: TrashedDeletion) {
  const values = {
    folder: parentDirectory(deletion.original) || t('settings.trash.root'),
    date: formatDateTime(deletion.deletedAtMs),
  };
  return isFolder(deletion)
    ? t('settings.trash.deletedFolderAt', { ...values, files: formatTrashCount(deletion.files) })
    : t('settings.trash.deletedAt', values);
}

interface TrashedNotesProps {
  workspacePath: string;
  onRestored: () => void;
}

export function TrashedNotes({ workspacePath, onRestored }: TrashedNotesProps) {
  const [page, setPage] = useState(0);
  const [trash, setTrash] = useState<TrashPage | null>(null);
  const [restoring, setRestoring] = useState(false);

  const load = useCallback((index: number) => {
    void listTrash(workspacePath, index * PAGE_SIZE, PAGE_SIZE)
      .then((next) => {
        if (next.deletions.length === 0 && index > 0) setPage(index - 1);
        else setTrash(next);
      })
      .catch((error) => console.error('Failed to list the trash', error));
  }, [workspacePath]);

  useEffect(() => load(page), [load, page]);

  const restore = (id: string) => {
    if (restoring) return;
    setRestoring(true);
    void restoreDeletion(workspacePath, id)
      .then(() => {
        onRestored();
        load(page);
      })
      .catch((error) => console.error('Failed to restore from trash', error))
      .finally(() => setRestoring(false));
  };

  if (!trash || trash.total === 0) return null;
  const first = page * PAGE_SIZE;
  const pages = Math.ceil(trash.total / PAGE_SIZE);

  return (
    <Section title={t('settings.trash.filesSection')}>
      {trash.deletions.map((deletion) => (
        <Row
          key={deletion.id}
          label={isFolder(deletion) ? fileName(deletion.original) : fileStem(deletion.original)}
          description={deletionDescription(deletion)}
        >
          <TextButton disabled={restoring} onClick={() => restore(deletion.id)}>
            {t('settings.trash.restore')}
          </TextButton>
        </Row>
      ))}
      {pages > 1 ? (
        <Row
          label={t('settings.trash.pageRange', {
            from: first + 1,
            to: first + trash.deletions.length,
            total: trash.total,
          })}
        >
          <TextButton disabled={page === 0} onClick={() => setPage(page - 1)}>
            {t('settings.trash.previous')}
          </TextButton>
          <TextButton disabled={page >= pages - 1} onClick={() => setPage(page + 1)}>
            {t('settings.trash.next')}
          </TextButton>
        </Row>
      ) : null}
    </Section>
  );
}
