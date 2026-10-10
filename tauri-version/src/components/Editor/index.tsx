import { lazy, Suspense, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { EditorView, ViewUpdate } from '@codemirror/view';
import { EditorNotice } from './EditorNotice';
import { t } from '../../i18n';
import { useEditorDoc } from './hooks/useEditorDoc';
import { useEditorExtensions } from './extensions';
import { documentSync } from './extensions/documentSync';
import { registerEditor } from './openEditors';
import type { OpenedDocument } from '../../modules/documents/documentGateway';
import { invalidateWikiLinks } from './extensions/livePreviewPlugin';
import { EditorContent } from './EditorContent';
import type { CodeMirrorFieldRef } from '../Common/CodeMirrorField';
import { useSettingsStore } from '../../modules/settings';
import { BookPage } from '../BookPage/BookPage';
const BookReader = lazy(() => import('../Reader/BookReader')
  .then((module) => ({ default: module.BookReader })));
import { useEditorDocContent } from './hooks/useEditorDocContent';
import { useEditorViewSetup } from './hooks/useEditorViewSetup';
import { useEditorBookPage } from './hooks/useEditorBookPage';
import { useEditorReader } from './hooks/useEditorReader';
import { useEditorMetadata } from './hooks/useEditorMetadata';
import { collectBookQuoteRefs, parseReaderQuoteHref } from '../../modules/docs/bookQuotes';
import { findReaderQuotes, readerQuoteTextRange } from './extensions/readerQuote/constructs';
import type { EditorProps } from './types';
import './Editor.css';

export function Editor({
  filePath,
  documentId,
  workspacePath,
  initialViewState,
  viewStateReady,
  onViewStateChange,
  onFileMissing,
  resolveWikiLinks,
  linkRevision,
  onOpenWikiLink,
  onOpenExternalUrl,
  revealOffset,
  inactive = false,
  onBodyViewChange,
}: EditorProps) {
  const titleRef = useRef<CodeMirrorFieldRef>(null);
  const bodyRef = useRef<EditorView>(null);
  useEffect(() => () => onBodyViewChange?.(null), [onBodyViewChange]);

  const {
    opened,
    title,
    renameTo,
    missing,
    openError,
    saveError,
    retrySave,
    reportFailure,
  } = useEditorDoc(filePath, workspacePath);
  const [resynced, setResynced] = useState<OpenedDocument | null>(null);
  const loaded = resynced ?? opened;
  const isReady = loaded !== null;

  const notePathRef = useRef(filePath);
  useEffect(() => {
    notePathRef.current = filePath;
  }, [filePath]);
  const currentNotePath = useCallback(() => notePathRef.current, []);

  const commitTitle = async () => {
    if (!document.hasFocus()) return;
    const draft = titleRef.current?.view?.state.doc.toString();
    if (draft === undefined) return;
    if (!await renameTo(draft)) titleRef.current?.setValue(title);
  };

  const { config } = useSettingsStore();
  const editorSettings = config?.editor;
  const smartDashes = editorSettings?.smartDashes ?? true;
  const listCallouts = editorSettings?.listCallouts ?? true;
  const autoLinkTitle = editorSettings?.autoLinkTitle ?? true;
  const linkSuggest = editorSettings?.linkSuggest ?? true;
  const linkSuggestMinChars = editorSettings?.linkSuggestMinChars ?? 2;

  const { initialBody, docContent, setDocContent } = useEditorDocContent(loaded?.text ?? '');
  const readText = useCallback(
    () => bodyRef.current?.state.doc.toString() ?? loaded?.text ?? '',
    [loaded],
  );

  const bookPage = useEditorBookPage({
    readText,
    filePath,
    workspacePath,
    docContent,
    setDocContent,
  });

  const reader = useEditorReader({
    readText,
    filePath,
    title,
    bookFile: bookPage.bookFile,
    bookAttached: bookPage.bookAttached,
    pagesFm: typeof bookPage.metadata?.pages === 'string' ? bookPage.metadata.pages : undefined,
    readerPosition: bookPage.resolved.readerPosition,
    onOpenExternalUrl,
  });

  const {
    containerRef,
    selection,
    handleCreate,
    handleUpdate,
    ready,
  } = useEditorViewSetup({
    documentId,
    viewStateReady,
    initialViewState,
    revealOffset,
    path: currentNotePath,
    isReady,
    initialBody,
    onViewStateChange,
    setDocContent,
    inactive,
  });

  const clientID = useMemo(() => crypto.randomUUID(), []);
  const syncExtension = useMemo(() => (loaded ? documentSync({
    path: currentNotePath,
    startVersion: loaded.version,
    clientID,
    onResync: setResynced,
    onFailure: reportFailure,
  }) : []), [clientID, currentNotePath, loaded, reportFailure]);

  const baseExtensions = useEditorExtensions(
    resolveWikiLinks,
    onOpenWikiLink,
    reader.handleOpenExternalUrl,
    smartDashes,
    listCallouts,
    autoLinkTitle,
    workspacePath,
    reader.handleReadBookCallout,
    currentNotePath,
    linkSuggest,
    linkSuggestMinChars,
    config?.plugins,
  );
  const extensions = useMemo(
    () => (isReady ? [baseExtensions, syncExtension] : []),
    [baseExtensions, isReady, syncExtension],
  );

  useEffect(() => {
    bodyRef.current?.dispatch({ effects: invalidateWikiLinks.of() });
  }, [linkRevision]);

  const quoteRefs = useMemo(
    () => collectBookQuoteRefs(docContent || initialBody),
    [docContent, initialBody],
  );

  const handleQuoteRefClick = useCallback((cfi: string) => {
    reader.handleCloseReader();
    requestAnimationFrame(() => {
      const view = bodyRef.current;
      if (!view) return;
      const span = findReaderQuotes(view.state.doc)
        .find((quote) => parseReaderQuoteHref(quote.href)?.cfi === cfi);
      if (!span) return;
      const text = readerQuoteTextRange(span);
      view.dispatch({
        selection: { anchor: text.from, head: text.to },
        scrollIntoView: true,
      });
      view.focus();
    });
  }, [reader.handleCloseReader]);

  const metadata = useEditorMetadata({
    filePath,
    docContent: docContent || initialBody,
    bodyRef,
  });

  const unregisterEditor = useRef<(() => void) | null>(null);
  useEffect(() => () => unregisterEditor.current?.(), []);

  const handleCreateEditor = useCallback((view: EditorView) => {
    unregisterEditor.current?.();
    unregisterEditor.current = registerEditor(currentNotePath, view);
    onBodyViewChange?.(view);
    metadata.syncOnCreate(view);
    handleCreate(view);
  }, [currentNotePath, handleCreate, metadata.syncOnCreate, onBodyViewChange]);

  const handleEditorUpdate = useCallback((update: ViewUpdate) => {
    handleUpdate(update);
    metadata.syncFromView(update.view);
  }, [handleUpdate, metadata.syncFromView]);

  useEffect(() => {
    if (missing) onFileMissing?.(filePath);
  }, [filePath, missing, onFileMissing]);

  const editorContent = (
    <EditorContent
        key={resynced ? `resync-${resynced.version}` : 'opened'}
        titleRef={titleRef}
        bodyRef={bodyRef}
        title={title}
        onCommitTitle={() => { void commitTitle(); }}
        initialBody={initialBody}
        selection={selection}
        extensions={extensions}
        autoLinkTitle={autoLinkTitle}
        onCreateEditor={handleCreateEditor}
        onUpdate={handleEditorUpdate}
        hasFrontmatter={metadata.hasFrontmatter}
        metadataExpanded={metadata.metadataExpanded}
        onToggleMetadata={metadata.toggleMetadata}
      />
  );

  if (openError) throw openError;

  const containerClass = bookPage.pageLayout
    ? 'q-editor-container q-editor-container--book'
    : 'q-editor-container';

  return (
    <div ref={containerRef} className={containerClass}>
      {saveError !== null && (
        <EditorNotice
          message={t('editor.saveFailed', { reason: saveError })}
          actionLabel={t('editor.saveRetry')}
          onAction={retrySave}
        />
      )}
      {ready ? (
        <BookPage
          hasCover={bookPage.hasCover}
          isBook={bookPage.isBook}
          hasBookFile={bookPage.bookAttached}
          workspacePath={workspacePath}
          pageCoverUrl={bookPage.resolved.pageCoverUrl}
          bookCoverUrl={bookPage.resolved.bookCoverUrl}
          pageCoverPosition={bookPage.resolved.pageCoverPosition}
          coverUploading={bookPage.coverUploading}
          bookBusy={bookPage.bookBusy}
          onReplacePageCover={bookPage.handleReplacePageCover}
          onRandomPageCover={bookPage.handleRandomPageCover}
          onRemovePageCover={bookPage.handleRemovePageCover}
          onReplaceBookCover={bookPage.handleReplaceBookCover}
          onRemoveBookCover={bookPage.handleRemoveBookCover}
          onPasteBookCover={bookPage.handlePasteBookCover}
          onPageCoverPositionChange={bookPage.handlePageCoverPositionChange}
          onUploadBook={() => { void bookPage.handleUploadBook(); }}
          onReadBook={reader.handleReadBook}
        >
          {editorContent}
        </BookPage>
      ) : (
        <div className="q-editor-loading" aria-busy="true" />
      )}
      {reader.readerSession ? (
        <Suspense fallback={null}>
        <BookReader
          workspacePath={workspacePath}
          bookFile={reader.readerSession.bookFile}
          title={reader.readerSession.title}
          pagesFm={reader.readerSession.pagesFm}
          initialCfi={reader.readerSession.initialCfi}
          peek={reader.readerSession.peek}
          onClose={reader.handleCloseReader}
          onPagesChange={
            reader.readerSession.progressPagePath
              ? reader.handleReaderPagesChange
              : undefined
          }
          onQuote={reader.handleReaderQuote}
          quotes={quoteRefs}
          onQuoteRefClick={handleQuoteRefClick}
        />
        </Suspense>
      ) : null}
    </div>
  );
}

