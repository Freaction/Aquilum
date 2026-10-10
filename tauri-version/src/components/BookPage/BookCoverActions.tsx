import { t } from '../../i18n';
import { Shuffle, Trash2, UnfoldVertical, Upload } from 'lucide';
import { Icon } from '../Common/Icon';
import { IconButton } from '../Common/IconButton';

interface BookCoverActionsProps {
  onReplacePageCover?: () => void;
  onRemovePageCover?: () => void;
  onRandomPageCover?: () => void;
  onToggleReposition?: () => void;
  repositioning?: boolean;
  canReposition?: boolean;
  uploading?: boolean;
}

export function BookCoverActions({
  onReplacePageCover,
  onRemovePageCover,
  onRandomPageCover,
  onToggleReposition,
  repositioning = false,
  canReposition = true,
  uploading = false,
}: BookCoverActionsProps) {
  return (
    <div className="q-floating-actions">
      <IconButton
        size="medium"
        label={t('book.replacePageCover')}
        disabled={uploading}
        onClick={onReplacePageCover}
      >
        <Icon icon={Upload} />
      </IconButton>
      <div className="q-floating-actions__divider" />
      <IconButton
        size="medium"
        label={t('book.randomCover')}
        disabled={uploading}
        onClick={onRandomPageCover}
      >
        <Icon icon={Shuffle} />
      </IconButton>
      {canReposition && (
        <>
          <div className="q-floating-actions__divider" />
          <IconButton
            size="medium"
            label={repositioning ? t('book.done') : t('book.coverPosition')}
            aria-pressed={repositioning}
            onClick={onToggleReposition}
          >
            <Icon icon={UnfoldVertical} />
          </IconButton>
        </>
      )}
      <div className="q-floating-actions__divider" />
      <IconButton
        size="medium"
        label={t('book.removePageCover')}
        disabled={uploading}
        onClick={onRemovePageCover}
      >
        <Icon icon={Trash2} />
      </IconButton>
    </div>
  );
}
