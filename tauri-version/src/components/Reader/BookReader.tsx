import { t } from '../../i18n';
import { matchesShortcut, SHORTCUTS } from '../../config/shortcuts';
import { useEffect, useMemo, useRef, useState } from 'react';
import { ChevronLeft, ChevronRight, Settings2, X } from 'lucide';
import { Icon } from '../Common/Icon';
import { Button } from '../Common/Button';
import { IconButton } from '../Common/IconButton';
import { useSettingsPersist } from '../Settings/useSettingsPersist';
import { useStableCallback } from '../../hooks/useStableCallback';
import { FoliateReaderEngine, prefetchFoliate } from './ReaderEngine';
import { ReaderSettingsPopover } from './ReaderSettingsPopover';
import { DEFAULT_READER_SETTINGS } from '../../modules/settings';
import { loadBookFile } from '../../modules/docs/books';
import type { BookQuoteRef } from '../../modules/docs/bookQuotes';
import {
  currentFromFraction,
  formatPages,
  formatSyntheticPages,
  parsePages,
  type Pages,
} from '../../modules/docs/bookProgress';
import {
  flushReaderState,
  hydrateReaderState,
  resolveReaderOpenTarget,
  setReaderState,
} from '../../modules/docs/readerState';
import './BookReader.css';

interface BookReaderProps {
  workspacePath: string | null;
  bookFile: string;
  title?: string;
  pagesFm?: string;
  initialCfi?: string;
  peek?: boolean;
  quotes?: BookQuoteRef[];
  onClose: () => void;
  onPagesChange?: (pages: string, extra?: { cfi: string; readPercent: number }) => void;
  onQuote?: (text: string, cfi: string) => void;
  onQuoteRefClick?: (cfi: string) => void;
}

function pagesFromFm(pagesFm?: string): Pages {
  return parsePages(pagesFm) ?? { current: 0, total: 1 };
}

export function BookReader({
  workspacePath,
  bookFile,
  title,
  pagesFm,
  initialCfi,
  peek,
  quotes,
  onClose,
  onPagesChange,
  onQuote,
  onQuoteRefClick,
}: BookReaderProps) {
  const { config, persist } = useSettingsPersist();
  const readerSettings = config?.reader ?? DEFAULT_READER_SETTINGS;
  const [settingsOpen, setSettingsOpen] = useState(false);

  const hostRef = useRef<HTMLDivElement | null>(null);
  const engineRef = useRef<FoliateReaderEngine | null>(null);

  const quoteMarks = useMemo(
    () => (quotes ?? [])
      .filter((quote) => !quote.bookFile || quote.bookFile === bookFile)
      .map((quote) => ({ cfi: quote.cfi, label: quote.label })),
    [quotes, bookFile],
  );
  const lastWritten = useRef(pagesFm ?? '');
  const totalRef = useRef(pagesFromFm(pagesFm).total);

  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [pages, setPages] = useState<Pages>(() => pagesFromFm(pagesFm));
  const [selection, setSelection] = useState<{ text: string; cfi: string } | null>(null);

  const writePages = useStableCallback((next: Pages, cfi?: string) => {
    if (!onPagesChange) return;
    const formatted = formatPages(next.current, next.total);
    const readPercent = next.total > 0 ? Math.round((next.current / next.total) * 100) : 0;
    if (formatted === lastWritten.current && !cfi) return;
    lastWritten.current = formatted;
    onPagesChange(formatted, cfi ? { cfi, readPercent } : undefined);
  });

  const readOpenInputs = useStableCallback(() => ({ pagesFm, quoteMarks, readerSettings }));
  const openQuoteRef = useStableCallback((cfi: string) => onQuoteRefClick?.(cfi));

  const handleKeyDown = useStableCallback((event: KeyboardEvent) => {
    const engine = engineRef.current;
    if (event.key === 'Escape') {
      event.preventDefault();
      if (settingsOpen) setSettingsOpen(false);
      else onClose();
      return;
    }
    if (settingsOpen || !engine) return;
    if (matchesShortcut(event, SHORTCUTS.READER_NEXT_PAGE) || event.key === 'PageDown' || event.key === ' ') {
      event.preventDefault();
      void engine.next();
    }
    if (matchesShortcut(event, SHORTCUTS.READER_PREVIOUS_PAGE) || event.key === 'PageUp') {
      event.preventDefault();
      void engine.prev();
    }
  });

  useEffect(() => {
    const host = hostRef.current;
    if (!host) return;

    const engine = new FoliateReaderEngine(host);
    engine.applyStyle(readOpenInputs().readerSettings);
    engineRef.current = engine;

    const unsubscribe = engine.onRelocate(({ fraction, cfi }) => {
      if (typeof fraction !== 'number' || !Number.isFinite(fraction)) return;
      const total = totalRef.current;
      const current = currentFromFraction(fraction, total);
      const next = { current, total };
      setPages((prev) => (prev.current === current && prev.total === total ? prev : next));
      writePages(next, cfi);
      if (!peek) setReaderState(bookFile, current, cfi);
    });
    const unsubSelection = engine.onSelection(setSelection);
    const unsubQuoteRef = engine.onQuoteRef(openQuoteRef);

    let cancelled = false;
    setLoading(true);
    setError(null);
    setSelection(null);

    void (async () => {
      try {
        const [, loaded] = await Promise.all([
          prefetchFoliate(),
          loadBookFile(workspacePath, bookFile),
          hydrateReaderState(workspacePath, bookFile),
        ]);
        if (cancelled) return;

        const latestPagesFm = readOpenInputs().pagesFm;
        const { pages: initial, formatted } = formatSyntheticPages(latestPagesFm, loaded.byteLength);
        totalRef.current = initial.total;
        lastWritten.current = formatted;
        setPages(initial);
        if (latestPagesFm !== formatted) writePages(initial);

        await engine.open(loaded.file, {
          lastLocation: resolveReaderOpenTarget(bookFile, initial, initialCfi),
        });
        await engine.setQuotes(readOpenInputs().quoteMarks);
      } catch (err) {
        if (cancelled) return;
        console.error('Failed to open book', err);
        setError(err instanceof Error ? err.message : t('reader.openFailed'));
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();

    window.addEventListener('keydown', handleKeyDown);

    return () => {
      cancelled = true;
      window.removeEventListener('keydown', handleKeyDown);
      unsubscribe();
      unsubSelection();
      unsubQuoteRef();
      engine.close();
      engineRef.current = null;
      void flushReaderState();
    };
  }, [bookFile, workspacePath, initialCfi, peek, handleKeyDown, openQuoteRef, readOpenInputs, writePages]);

  useEffect(() => {
    void engineRef.current?.setQuotes(quoteMarks);
  }, [quoteMarks]);

  useEffect(() => {
    engineRef.current?.applyStyle(readerSettings);
  }, [readerSettings]);

  const patchReader = (partial: Partial<typeof readerSettings>) => {
    if (!config) return;
    persist({ ...config, reader: { ...config.reader, ...partial } });
  };

  const progressPct = pages.total > 0 ? (pages.current / pages.total) * 100 : 0;

  const applyQuote = () => {
    if (!selection || !onQuote) return;
    onQuote(selection.text, selection.cfi);
    setSelection(null);
  };

  return (
    <div className="q-book-reader" role="dialog" aria-modal="true" aria-label={t('reader.title')}>
      <header className="q-book-reader-bar">
        <div className="q-book-reader-heading">
          <div className="q-book-reader-title">{title || t('reader.book')}</div>
          <div className="q-book-reader-pages" aria-live="polite">
            {pages.current}/{pages.total}
          </div>
        </div>
        <div className="q-book-reader-actions">
          <IconButton
            size="medium"
            label={t('reader.settings')}
            aria-expanded={settingsOpen}
            onClick={() => setSettingsOpen((open) => !open)}
          >
            <Icon icon={Settings2} />
          </IconButton>
          <IconButton
            size="medium"
            label={t('reader.close')}
            onClick={onClose}
          >
            <Icon icon={X} />
          </IconButton>
        </div>
        {settingsOpen ? (
          <ReaderSettingsPopover
            settings={readerSettings}
            onChange={patchReader}
            onClose={() => setSettingsOpen(false)}
          />
        ) : null}
      </header>

      <div className="q-book-reader-body">
        <button
          type="button"
          className="q-book-reader-nav q-book-reader-nav--prev"
          title={t('reader.back')}
          aria-label={t('reader.prevPage')}
          onClick={() => void engineRef.current?.prev()}
        >
          <Icon icon={ChevronLeft} />
        </button>
        <button
          type="button"
          className="q-book-reader-nav q-book-reader-nav--next"
          title={t('reader.forward')}
          aria-label={t('reader.nextPage')}
          onClick={() => void engineRef.current?.next()}
        >
          <Icon icon={ChevronRight} />
        </button>

        {loading && <div className="q-book-reader-loading" aria-busy="true" />}
        {error && <div className="q-book-reader-status q-book-reader-status--error">{error}</div>}
        <div
          ref={hostRef}
          className="q-book-reader-host"
          hidden={Boolean(error)}
          data-loading={loading ? 'true' : undefined}
        />

        {selection && onQuote ? (
          <div className="q-book-reader-selection-toolbar">
            <p className="q-book-reader-selection-preview">{selection.text}</p>
            <Button size="s" className="q-book-reader-selection-action" onClick={applyQuote}>
              {t('reader.quote')}
            </Button>
          </div>
        ) : null}
      </div>

      <footer className="q-book-reader-footer">
        <div
          className="q-book-reader-progress"
          role="progressbar"
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={Math.round(progressPct)}
          aria-label={t('reader.progress')}
        >
          <div className="q-book-reader-progress-fill" style={{ width: `${progressPct}%` }} />
        </div>
      </footer>
    </div>
  );
}
