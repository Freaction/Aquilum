import { t } from '../../i18n';
import { useEffect, useRef, useState, type KeyboardEvent } from 'react';
import { Upload } from 'lucide';
import { Icon } from '../Common/Icon';
import { IconButton } from '../Common/IconButton';
import { BookCoverOverlay } from '../Decorations/BookCoverOverlay';
import { CoverImage } from './CoverImage';
import { mediaKindForMime } from '../../modules/docs/imageEmbeds';

interface BookThumbnailProps {
  src: string;
  uploading?: boolean;
  onReplace?: () => void;
  onRemove?: () => void;
  onPasteImage?: (file: File) => void;
}

function clipboardImageOf(transfer: DataTransfer | null): File | null {
  if (!transfer) return null;
  return Array.from(transfer.files).find((file) => mediaKindForMime(file.type) === 'image') ?? null;
}

export function BookThumbnail({
  src,
  uploading = false,
  onReplace,
  onRemove,
  onPasteImage,
}: BookThumbnailProps) {
  const wrapperRef = useRef<HTMLDivElement>(null);
  const [hovered, setHovered] = useState(false);
  const [focused, setFocused] = useState(false);

  useEffect(() => {
    if (!focused || !onPasteImage) return;
    const handlePaste = (event: ClipboardEvent) => {
      const file = clipboardImageOf(event.clipboardData);
      if (!file) return;
      event.preventDefault();
      onPasteImage(file);
    };
    document.addEventListener('paste', handlePaste);
    return () => document.removeEventListener('paste', handlePaste);
  }, [focused, onPasteImage]);

  const handleKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key === 'Backspace' || event.key === 'Delete') {
      event.preventDefault();
      onRemove?.();
      return;
    }
    if (event.key === 'Escape') wrapperRef.current?.blur();
  };

  return (
    <div
      ref={wrapperRef}
      className="q-book-thumbnail-wrapper"
      tabIndex={0}
      aria-label={t('book.coverAria')}
      onPointerEnter={() => setHovered(true)}
      onPointerLeave={() => setHovered(false)}
      onFocus={() => setFocused(true)}
      onBlur={() => setFocused(false)}
      onKeyDown={handleKeyDown}
    >
      <CoverImage src={src} className="q-book-thumbnail-image" />
      <div className="q-book-thumbnail-shine" aria-hidden="true" />
      <BookCoverOverlay className="q-book-thumbnail-color-wash" />
      {hovered && (
        <div className="q-book-thumbnail-action">
          <IconButton
            variant="white"
            size="medium"
            label={t('book.replaceBookCover')}
            disabled={uploading}
            onClick={onReplace}
          >
            <Icon icon={Upload} />
          </IconButton>
        </div>
      )}
    </div>
  );
}
