import { memo, useEffect, useRef, type MouseEvent } from 'react';
import { History, CalendarDays } from 'lucide';
import { CalendarPanel } from '../Calendar/CalendarPanel';
import { DEFAULT_DAILY_NOTES_SETTINGS } from '../../modules/dailyNotes';
import { Icon } from '../Common/Icon';
import { t } from '../../i18n';
import {
  enabledSidebarMethods,
  isGraphAnalysisMethod,
  useDocumentAnalysis,
  type AnalysisResult,
  type SidebarSourceMethod,
} from '../../modules/analysis';
import {
  useDocumentLinks,
  type Backlink,
  type LinkDisposition,
  type LinkMode,
  type OutgoingLink,
} from '../../modules/links';
import { isMarkdownPath } from '../../modules/documents/fileGateway';
import { openExternalUrl } from '../../modules/openExternalUrl';
import { DEFAULT_PLUGIN_SETTINGS, useSettingsStore } from '../../modules/settings';
import { useWikixivSources, setWikiHover, clearWikiHover } from '../../modules/wikixiv';
import { isEmptyTabPath } from '../../modules/ui-state';
import { useLocalState } from '../../modules/workspace/uiPersist';
import { Chip } from '../Common/Chip';
import { IconButton } from '../Common/IconButton';
import { useHorizontalWheelScroll } from '../Common/useHorizontalWheelScroll';
import {
  BacklinksIcon,
  GraphAnalysisIcon,
  OutgoingLinksIcon,
} from '../Icons/LinkIcons';
import { HistoryList } from '../History/HistoryList';
import { SidebarDocumentItem } from './SidebarDocumentItem';
import './BacklinksPanel.css';

type PanelMode = LinkMode | 'analysis' | 'history' | 'calendar';

const PANEL_MODES: readonly string[] = ['backlinks', 'outgoing', 'analysis', 'history', 'calendar'] satisfies PanelMode[];

function resolvePanelMode(stored: string, sourcesAvailable: boolean, calendarEnabled: boolean): PanelMode {
  if (!PANEL_MODES.includes(stored)) return 'backlinks';
  if (stored === 'calendar' && !calendarEnabled) return 'backlinks';
  if (stored === 'analysis' && !sourcesAvailable) return 'backlinks';
  return stored as PanelMode;
}

interface BacklinksPanelProps {
  onOpenNote: (path: string) => void;
  workspacePath: string | null;
  documentPath: string | null;
  activeTabId: string | null;
  indexReady: boolean;
  indexRevision: number;
  isOpen: boolean;
  onOpenBacklink: (backlink: Backlink, disposition: LinkDisposition) => void;
  onOpenOutgoing: (link: OutgoingLink, disposition: LinkDisposition) => void;
  onOpenAnalysis: (result: AnalysisResult, disposition: LinkDisposition) => void;
}

function dispositionFromEvent(event: MouseEvent<Element>): LinkDisposition {
  return event.ctrlKey || event.metaKey ? 'new-tab' : 'current';
}

function sourceMethodLabel(method: SidebarSourceMethod): string {
  return t(`analysis.methods.${method}`);
}

function BacklinksPanelComponent({
  workspacePath,
  documentPath,
  activeTabId,
  indexReady,
  indexRevision,
  isOpen,
  onOpenBacklink,
  onOpenOutgoing,
  onOpenAnalysis,
  onOpenNote,
}: BacklinksPanelProps) {
  const { config } = useSettingsStore();
  const sidebarMethods = enabledSidebarMethods(config?.analysis);
  const sourcesAvailable = sidebarMethods.length > 0;
  const [storedMode, setMode] = useLocalState<string>('aquilum_backlinks_mode', 'backlinks');
  const calendarSettings = config?.plugins?.calendar ?? DEFAULT_PLUGIN_SETTINGS.calendar;
  const mode = resolvePanelMode(storedMode, sourcesAvailable, calendarSettings.enabled);
  const [sourceMethod, setSourceMethod] = useLocalState<SidebarSourceMethod>(
    'aquilum_analysis_method',
    'bm25f',
  );
  const activeSourceMethod = sidebarMethods.includes(sourceMethod)
    ? sourceMethod
    : sidebarMethods[0] ?? sourceMethod;
  const chipRowRef = useRef<HTMLDivElement>(null);

  const hasDocument = Boolean(
    workspacePath && documentPath && !isEmptyTabPath(documentPath),
  );
  const isAnalysis = mode === 'analysis';
  const isHistory = mode === 'history';
  const isCalendar = mode === 'calendar';
  const isWixiv = isAnalysis && activeSourceMethod === 'wixiv';
  useHorizontalWheelScroll(chipRowRef, isAnalysis, sidebarMethods.length);

  useEffect(() => {
    if (!isAnalysis) clearWikiHover();
    return clearWikiHover;
  }, [isAnalysis]);
  const isGraphAnalysis = isAnalysis && isGraphAnalysisMethod(activeSourceMethod);
  const linkMode: LinkMode = mode === 'outgoing' ? 'outgoing' : 'backlinks';
  const { result: linkResult, loading: linksLoading } = useDocumentLinks(
    linkMode,
    workspacePath,
    documentPath,
    indexReady,
    indexRevision,
    isOpen && hasDocument && !isAnalysis && !isHistory && !isCalendar,
  );
  const {
    items: analysisItems,
    failed: analysisFailed,
    loading: analysisLoading,
  } = useDocumentAnalysis(
    isGraphAnalysis ? activeSourceMethod : 'bm25f',
    workspacePath,
    documentPath,
    indexReady,
    indexRevision,
    isOpen && hasDocument && isGraphAnalysis,
  );
  const {
    hits: wikixivHits,
    offline: wikixivOffline,
    insufficientText: wikixivInsufficient,
    failed: wikixivFailed,
    loading: wikixivLoading,
  } = useWikixivSources(documentPath, isOpen && hasDocument && isWixiv);

  const isBacklinks = mode === 'backlinks';
  const heading = t(isWixiv
    ? 'wikixiv.heading'
    : isGraphAnalysis
      ? 'analysis.noteColumn'
      : isBacklinks
        ? 'backlinks.mentionsTitle'
        : 'backlinks.outgoingTitle');
  const empty = t(isWixiv
    ? 'wikixiv.empty'
    : isGraphAnalysis
      ? 'analysis.empty'
      : isBacklinks
        ? 'backlinks.empty'
        : 'backlinks.outgoingEmpty');
  const analysisReasonLabel = activeSourceMethod === 'adamicAdar'
    ? t('analysis.commonNeighbors')
    : t('analysis.commonTerms');
  const analysisValueLabel = t('analysis.scoreColumn');
  const loading = isWixiv
    ? wikixivLoading
    : isGraphAnalysis
      ? analysisLoading
      : linksLoading;

  return (
    <aside
      className={`q-collapsible-panel q-backlinks ${isOpen ? '' : 'is-collapsed'}`}
      aria-hidden={!isOpen}
      aria-label={t('backlinks.panelTitle')}
      data-side="Right"
    >
      <div className="q-panel-header" data-tauri-drag-region>
        <div className="q-backlinks__mode-tabs" role="group" aria-label={t('backlinks.modeLabel')}>
          <IconButton
            label={t('backlinks.backlinksTab')}
            className="q-backlinks__mode-button"
            aria-pressed={isBacklinks}
            onClick={() => setMode('backlinks')}
          >
            <BacklinksIcon />
          </IconButton>
          <IconButton
            label={t('backlinks.outgoingTab')}
            className="q-backlinks__mode-button"
            aria-pressed={mode === 'outgoing'}
            onClick={() => setMode('outgoing')}
          >
            <OutgoingLinksIcon />
          </IconButton>
          {sourcesAvailable && (
            <IconButton
              label={t('analysis.tab')}
              className="q-backlinks__mode-button"
              aria-pressed={isAnalysis}
              onClick={() => setMode('analysis')}
            >
              <GraphAnalysisIcon />
            </IconButton>
          )}
          <IconButton
            label={t('backlinks.historyTab')}
            className="q-backlinks__mode-button"
            aria-pressed={isHistory}
            onClick={() => setMode('history')}
          >
            <Icon icon={History} />
          </IconButton>
          {calendarSettings.enabled && <IconButton label={t('calendar.title')} className="q-backlinks__mode-button" aria-pressed={isCalendar} onClick={() => setMode('calendar')}>
            <Icon icon={CalendarDays} />
          </IconButton>}
        </div>
      </div>

      {isCalendar && isOpen && <CalendarPanel indexRevision={indexRevision} workspacePath={workspacePath} documentPath={documentPath} settings={config?.dailyNotes ?? DEFAULT_DAILY_NOTES_SETTINGS} calendarSettings={calendarSettings} onOpenNote={onOpenNote} />}

      {isAnalysis && (
        <div className="q-backlinks__chips">
          <div
            ref={chipRowRef}
            className="q-backlinks__chip-row"
            role="group"
            aria-label={t('analysis.methodLabel')}
          >
            {sidebarMethods.map((method) => (
              <Chip
                key={method}
                label={sourceMethodLabel(method)}
                selected={activeSourceMethod === method}
                onClick={() => setSourceMethod(method)}
              />
            ))}
          </div>
        </div>
      )}

      {hasDocument && isHistory && documentPath && isMarkdownPath(documentPath) && (
        <div className="q-backlinks__content">
          <header className="q-backlinks__header">
            <h2>{t('history.title')}</h2>
          </header>
          <HistoryList path={documentPath} tabId={activeTabId} isOpen={isOpen} />
        </div>
      )}

      {hasDocument && !isHistory && !isCalendar && (
        <div className="q-backlinks__content">
          <header className="q-backlinks__header">
            <h2>{heading}</h2>
            {isGraphAnalysis && (
              <span className="q-panel-counter">{analysisValueLabel}</span>
            )}
          </header>
          {!loading && isGraphAnalysis && analysisFailed && (
            <p className="q-panel-empty">{t('analysis.error')}</p>
          )}
          {!loading && isWixiv && wikixivOffline && (
            <p className="q-panel-empty q-panel-empty--wrap">{t('wikixiv.offline')}</p>
          )}
          {!loading && isWixiv && !wikixivOffline && wikixivFailed && (
            <p className="q-panel-empty">{t('wikixiv.error')}</p>
          )}
          {!loading && isWixiv && !wikixivOffline && !wikixivFailed && wikixivInsufficient && (
            <p className="q-panel-empty q-panel-empty--wrap">{t('wikixiv.insufficient')}</p>
          )}
          {!loading && !analysisFailed && !isWixiv && (isGraphAnalysis ? analysisItems?.length === 0 : linkResult?.items.length === 0) && (
            <p className="q-panel-empty">{empty}</p>
          )}
          {!loading && isWixiv && !wikixivOffline && !wikixivFailed && !wikixivInsufficient && wikixivHits.length === 0 && (
            <p className="q-panel-empty">{empty}</p>
          )}
          {!isAnalysis && linkResult?.mode === 'backlinks' && linkResult.items.map((link) => (
            <SidebarDocumentItem
              key={`${link.path}:${link.offset}`}
              label={link.title}
              onClick={(event) => onOpenBacklink(link, dispositionFromEvent(event))}
              onAuxClick={(event) => { if (event.button === 1) onOpenBacklink(link, 'new-tab'); }}
            />
          ))}
          {!isAnalysis && linkResult?.mode === 'outgoing' && linkResult.items.map((link, index) => (
            <SidebarDocumentItem
              key={`${link.target}:${index}`}
              label={link.title}
              onClick={(event) => onOpenOutgoing(link, dispositionFromEvent(event))}
              onAuxClick={(event) => { if (event.button === 1) onOpenOutgoing(link, 'new-tab'); }}
            />
          ))}
          {isGraphAnalysis && analysisItems?.map((result) => {
            const showHover = () => setWikiHover({ filename: false, terms: result.reasons });
            return (
              <SidebarDocumentItem
                key={result.path}
                label={result.title}
                counter={result.confidence !== undefined
                  ? `${(result.confidence * 100).toFixed(1)}%`
                  : result.rawScore.toFixed(2)}
                title={result.reasons.length > 0
                  ? `${result.title}\n${analysisReasonLabel}: ${result.reasons.join(', ')}`
                  : result.title}
                onClick={(event) => onOpenAnalysis(result, dispositionFromEvent(event))}
                onAuxClick={(event) => { if (event.button === 1) onOpenAnalysis(result, 'new-tab'); }}
                onMouseEnter={showHover}
                onMouseLeave={clearWikiHover}
                onFocus={showHover}
                onBlur={clearWikiHover}
              />
            );
          })}
          {isWixiv && wikixivHits.map((hit) => {
            const showHover = () =>
              setWikiHover({ filename: hit.fromFilename, terms: hit.matchedTerms });
            return (
              <SidebarDocumentItem
                key={hit.url}
                label={hit.title}
                title={hit.snippet ? `${hit.title}\n\n${hit.snippet}` : hit.title}
                onClick={() => {
                  void openExternalUrl(hit.url).catch((error) => {
                    console.error('Failed to open Wiki url', error);
                  });
                }}
                onMouseEnter={showHover}
                onMouseLeave={clearWikiHover}
                onFocus={showHover}
                onBlur={clearWikiHover}
              />
            );
          })}
        </div>
      )}
    </aside>
  );
}

export const BacklinksPanel = memo(BacklinksPanelComponent);
