import { t } from '../../i18n';
import { forwardRef, useState, type ReactNode } from 'react';
import './BookPage.css';
import { Rays } from '../Decorations/Rays';
import { Star } from '../Decorations/Star';
import { CoverPattern } from '../Decorations/CoverPattern';
import { BookCoverActions } from './BookCoverActions';
import { BookThumbnail } from './BookThumbnail';
import defaultBookCover from '../../assets/covers/default-book-cover.webp';
import { Button } from '../Common/Button';
import { resolvePageCoverVisual } from '../../modules/docs/covers';
import { resolveVaultAssetUrl } from '../../modules/docs/vaultAssets';
import { CoverImage } from './CoverImage';
import { useCoverReposition } from './useCoverReposition';
import { useThemeMode } from '../../hooks/useThemeMode';

interface BookPageProps {
  children: ReactNode;
  pageCoverUrl?: string;
  bookCoverUrl?: string;
  pageCoverPosition?: string;
  workspacePath?: string | null;
  hasCover?: boolean;
  isBook?: boolean;
  hasBookFile?: boolean;
  coverUploading?: boolean;
  bookBusy?: boolean;
  onReplacePageCover?: () => void;
  onRandomPageCover?: () => void;
  onRemovePageCover?: () => void;
  onReplaceBookCover?: () => void;
  onRemoveBookCover?: () => void;
  onPasteBookCover?: (file: File) => void;
  onPageCoverPositionChange?: (position: string) => void;
  onUploadBook?: () => void;
  onReadBook?: () => void;
}

export const BookPage = forwardRef<HTMLDivElement, BookPageProps>(
  (
    {
      children,
      pageCoverUrl,
      bookCoverUrl,
      pageCoverPosition,
      workspacePath = null,
      hasCover = false,
      isBook = false,
      hasBookFile = false,
      coverUploading = false,
      bookBusy = false,
      onReplacePageCover,
      onRandomPageCover,
      onRemovePageCover,
      onReplaceBookCover,
      onRemoveBookCover,
      onPasteBookCover,
      onPageCoverPositionChange,
      onUploadBook,
      onReadBook,
    },
    ref,
  ) => {
    const active = hasCover || isBook;
    const themeMode = useThemeMode();
    const [coverHovered, setCoverHovered] = useState(false);
    const pageCover = resolvePageCoverVisual(workspacePath, pageCoverUrl);
    const pageCoverImage = pageCover.kind === 'image' ? pageCover.url : '';
    const resolvedBookCover = isBook
      ? resolveVaultAssetUrl(workspacePath, bookCoverUrl, defaultBookCover)
      : '';

    const reposition = useCoverReposition({
      enabled: hasCover && pageCover.kind === 'image',
      imageUrl: pageCoverImage,
      position: pageCoverPosition,
      onPositionChange: onPageCoverPositionChange,
    });

    const bookFileButton = (
      <Button disabled={bookBusy} onClick={hasBookFile ? onReadBook : onUploadBook}>
        {hasBookFile ? t('book.read') : t('book.upload')}
      </Button>
    );

    return (
      <div
        ref={ref}
        className={[
          'q-book-page',
          !active ? 'q-book-page--inactive' : '',
          hasCover ? 'q-book-page--has-cover' : '',
          isBook ? 'q-book-page--has-book' : '',
          reposition.repositioning ? 'q-book-page--repositioning-cover' : '',
        ].filter(Boolean).join(' ')}
      >
        {active && (
          <div className="q-book-hero">
            {hasCover && (
              <div
                ref={reposition.coverRef}
                className={`q-book-page-cover${reposition.repositioning ? ' q-book-page-cover--repositioning' : ''}`}
                onPointerDown={reposition.onPointerDown}
                onPointerMove={reposition.onPointerMove}
                onPointerUp={reposition.onPointerUp}
                onPointerCancel={reposition.onPointerUp}
                onPointerEnter={() => setCoverHovered(true)}
                onPointerLeave={() => setCoverHovered(false)}
              >
                {pageCover.kind === 'pattern' ? (
                  <CoverPattern id={pageCover.id} />
                ) : (
                  <CoverImage
                    ref={reposition.imageRef}
                    src={pageCover.url}
                    className="q-book-page-cover-image"
                    style={{ objectPosition: `50% ${reposition.y}%` }}
                    onLoad={reposition.measureOverflow}
                  />
                )}
                {reposition.repositioning && (
                  <div className="q-book-page-cover-frame" aria-hidden="true" />
                )}
                {themeMode === 'light' && (
                  <div className="q-book-page-cover-rays">
                    <Rays />
                  </div>
                )}
                {(coverHovered || reposition.repositioning || isBook) && (
                <div
                  className="q-book-page-controls"
                  onPointerDown={(event) => event.stopPropagation()}
                >
                  {(coverHovered || reposition.repositioning) && (
                    <BookCoverActions
                      uploading={coverUploading}
                      repositioning={reposition.repositioning}
                      canReposition={pageCover.kind === 'image'}
                      onReplacePageCover={onReplacePageCover}
                      onRandomPageCover={onRandomPageCover}
                      onRemovePageCover={onRemovePageCover}
                      onToggleReposition={reposition.toggleReposition}
                    />
                  )}
                  {isBook && bookFileButton}
                </div>
                )}
              </div>
            )}

            {isBook && !hasCover && (
              <div className="q-book-solo-controls">
                {bookFileButton}
              </div>
            )}

            {isBook && (
              <div className="q-book-thumbnail-rail">
                <div className="q-book-thumbnail-container">
                  <div className="q-book-star-layer">
                    <Star />
                  </div>
                  <BookThumbnail
                    src={resolvedBookCover}
                    uploading={coverUploading}
                    onReplace={onReplaceBookCover}
                    onRemove={onRemoveBookCover}
                    onPasteImage={onPasteBookCover}
                  />
                </div>
              </div>
            )}
          </div>
        )}

        <div className={`q-book-content-wrapper ${!active ? 'q-book-content-wrapper--inactive' : ''}`}>
          <div className="q-book-editor-container">
            {children}
          </div>
        </div>
      </div>
    );
  }
);
