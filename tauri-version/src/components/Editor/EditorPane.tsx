import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { EditorView } from '@codemirror/view';
import { Editor } from './index';
import { matchesShortcut, SHORTCUTS } from '../../config/shortcuts';
import { ErrorBoundary } from '../Common/ErrorBoundary';
import { t } from '../../i18n';
import { EditorNotice } from './EditorNotice';
import { failureReason } from './noteOpenError';
import { resetSessionState, type StateFailure } from '../../modules/ui-state';
import {
  resolveWikiLinks,
  type LinkDisposition,
  type WikiLinkResolver,
} from '../../modules/links';
import type { ViewState, SessionTab } from '../../modules/ui-state';
import { readDocument } from '../../modules/documents/documentGateway';
import { exportNoteToPdf } from '../../modules/export';
import { fileName, fileStem, samePath } from '../../modules/paths';
import { closeVersion, useOpenedVersion } from '../../modules/history';
import { HistoryView } from '../History/HistoryView';
import { EditorToolbar } from './EditorToolbar';
import { PageSearchBar } from './PageSearchBar';

interface EditorPaneProps {
  canGoBack: boolean; canGoForward: boolean; onNavigate: (delta: -1 | 1) => void;
  tab: SessionTab;
  inactive: boolean;
  remountNonce: string;
  workspacePath: string | null;
  ensureLinksReady: () => Promise<void>;
  linkRevision: number;
  loadedView: (documentId: string) => ViewState | null;
  isViewLoaded: (documentId: string) => boolean;
  viewRevision: number;
  stateError: StateFailure | null;
  revealOffset?: number;
  onViewStateChange: (state: ViewState) => void;
  onFileMissing?: (path: string) => void;
  onOpenWikiLink: (target: string, disposition: LinkDisposition) => void;
  onOpenExternalUrl: (url: string) => void;
  focusMode: boolean;
  onToggleFocusMode: () => void;
}

export function EditorPane({
  canGoBack, canGoForward, onNavigate,
  tab,
  inactive,
  remountNonce,
  workspacePath,
  ensureLinksReady,
  linkRevision,
  loadedView,
  isViewLoaded,
  viewRevision,
  stateError,
  revealOffset,
  onViewStateChange,
  onFileMissing,
  onOpenWikiLink,
  onOpenExternalUrl,
  focusMode,
  onToggleFocusMode,
}: EditorPaneProps) {
  const [pageSearchOpen, setPageSearchOpen] = useState(false);
  const [pageSearchFocusRequest, setPageSearchFocusRequest] = useState(0);
  const [bodyView, setBodyView] = useState<EditorView | null>(null);
  const opened = useOpenedVersion(tab.tabId);
  const viewing = opened && samePath(opened.path, tab.path) ? opened : null;
  const viewingVersion = viewing !== null;

  useEffect(() => {
    if (opened && !viewing) closeVersion(tab.tabId);
  }, [opened, viewing, tab.tabId]);

  useEffect(() => {
    if (inactive || viewingVersion) return undefined;
    const onKeyDown = (event: KeyboardEvent) => {
      if (matchesShortcut(event, SHORTCUTS.PAGE_SEARCH)) {
        event.preventDefault();
        event.stopPropagation();
        setPageSearchOpen(true);
        setPageSearchFocusRequest((request) => request + 1);
      }
    };
    window.addEventListener('keydown', onKeyDown, true);
    return () => window.removeEventListener('keydown', onKeyDown, true);
  }, [inactive, viewingVersion]);
  const contextRef = useRef({ workspacePath, ensureLinksReady, path: tab.path });
  useEffect(() => {
    contextRef.current = { workspacePath, ensureLinksReady, path: tab.path };
  });
  const resolveForTab: WikiLinkResolver = useCallback(async (targets) => {
    const { workspacePath: root, ensureLinksReady: ensure, path } = contextRef.current;
    if (!root) return { paths: targets.map(() => null), complete: false };
    await ensure();
    return resolveWikiLinks(root, path, targets);
  }, []);

  const exportPdf = useCallback(() => {
    void readDocument(tab.path)
      .then(({ text }) => exportNoteToPdf({ title: fileStem(tab.path), markdown: text, workspacePath }))
      .catch((error) => console.error('Failed to export the note to PDF', error));
  }, [tab.path, workspacePath]);

  const documentId = tab.documentId;
  const viewStateReady = stateError !== null || (documentId ? isViewLoaded(documentId) : false);
  const initialViewState = useMemo(
    () => (documentId ? loadedView(documentId) : null),
    [documentId, loadedView, viewRevision, viewStateReady],
  );

  return (
    <ErrorBoundary resetKey={`${tab.tabId}:${tab.mountKey ?? ''}:${tab.path}`}>
      <div
        className={[
          'q-editor-pane',
          inactive ? 'q-offstage' : '',
        ].filter(Boolean).join(' ')}
        aria-hidden={inactive}
      >
        <EditorToolbar
          fileName={fileName(tab.path)}
          canGoBack={canGoBack}
          canGoForward={canGoForward}
          onNavigate={onNavigate}
          onSearch={() => setPageSearchOpen(true)}
          onExportPdf={exportPdf}
          focusMode={focusMode}
          onToggleFocusMode={onToggleFocusMode}
        />
        {stateError !== null && (
          <EditorNotice
            message={stateError.databaseBroken
              ? `${t('editor.stateUnavailable', { reason: failureReason(stateError.error) })} ${t('editor.resetUiStateWarning')}`
              : t('editor.stateUnavailable', { reason: failureReason(stateError.error) })}
            actionLabel={stateError.databaseBroken ? t('editor.resetUiState') : t('editor.restart')}
            onAction={() => {
              if (!stateError.databaseBroken) {
                window.location.reload();
                return;
              }
              resetSessionState(workspacePath).catch((error) => {
                console.error('Failed to reset the tab state', error);
              });
            }}
          />
        )}
        <PageSearchBar
          open={pageSearchOpen}
          focusRequest={pageSearchFocusRequest}
          view={bodyView}
          onClose={() => setPageSearchOpen(false)}
        />
        <div className="q-editor-stage">
          <div className="q-editor-stage__editor" inert={viewingVersion}>
            <Editor
              key={`${tab.tabId}:${tab.mountKey ?? ''}:${remountNonce}`}
              filePath={tab.path}
              documentId={documentId}
              workspacePath={workspacePath}
              initialViewState={initialViewState}
              viewStateReady={viewStateReady}
              onViewStateChange={onViewStateChange}
              onFileMissing={onFileMissing}
              resolveWikiLinks={resolveForTab}
              linkRevision={linkRevision}
              onOpenWikiLink={onOpenWikiLink}
              onOpenExternalUrl={onOpenExternalUrl}
              revealOffset={revealOffset}
              inactive={inactive}
              onBodyViewChange={setBodyView}
            />
          </div>
          {viewing ? <HistoryView tabId={tab.tabId} opened={viewing} /> : null}
        </div>
      </div>
    </ErrorBoundary>
  );
}


