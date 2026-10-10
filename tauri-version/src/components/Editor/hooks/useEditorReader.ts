import { useCallback, useEffect, useRef, useState } from 'react';
import { Text } from '@codemirror/state';

import { hasBookFile } from '../../../modules/docs/books';
import { formatBookQuote, parseReaderQuoteHref } from '../../../modules/docs/bookQuotes';
import {
  flushBookReaderMeta,
  persistBookReaderMeta,
} from '../../../modules/docs/persistBookReaderMeta';
import { findReaderQuotes } from '../extensions/readerQuote/constructs';
import { appendQuoteToDocument } from '../docMutations';
import type { BookCalloutReadRequest } from '../extensions/bookCallout';
import type { ReaderSession } from '../types';

function warmReaderEngine(): void {
  void import('../../Reader/ReaderEngine').then((module) => module.prefetchFoliate());
}

export function useEditorReader(options: {
  readText: () => string;
  filePath: string;
  title: string;
  bookFile?: string;
  bookAttached: boolean;
  pagesFm?: string;
  readerPosition?: string;
  onOpenExternalUrl: (url: string) => void;
}) {
  const {
    readText,
    filePath,
    title,
    bookFile,
    bookAttached,
    pagesFm,
    readerPosition,
    onOpenExternalUrl,
  } = options;

  const [readerSession, setReaderSession] = useState<ReaderSession | null>(null);
  const readerSessionRef = useRef<ReaderSession | null>(null);
  readerSessionRef.current = readerSession;

  const openReader = useCallback((session: ReaderSession) => {
    setReaderSession(session);
    warmReaderEngine();
  }, []);

  const handleOpenExternalUrl = useCallback((url: string) => {
    const parsed = parseReaderQuoteHref(url);
    if (parsed?.cfi) {
      const targetBook = parsed.bookFile ?? bookFile;
      if (targetBook && hasBookFile(targetBook)) {
        const sameBook = !parsed.bookFile || parsed.bookFile === bookFile;
        openReader({
          bookFile: targetBook,
          title: sameBook ? title : 'Книга',
          pagesFm: sameBook ? pagesFm : undefined,
          initialCfi: parsed.cfi,
          peek: true,
        });
        return;
      }
    }
    onOpenExternalUrl(url);
  }, [bookFile, onOpenExternalUrl, openReader, pagesFm, title]);

  const handleReadBookCallout = useCallback((request: BookCalloutReadRequest) => {
    if (!hasBookFile(request.bookFile) || !request.bookPagePath) return;
    openReader({
      bookFile: request.bookFile,
      title: request.title,
      pagesFm: request.pagesFm,
      progressPagePath: request.bookPagePath,
    });
  }, [openReader]);

  const handleReadBook = useCallback(() => {
    if (!bookAttached || !bookFile) return;
    openReader({
      bookFile,
      title,
      pagesFm,
      initialCfi: readerPosition,
      progressPagePath: filePath,
    });
  }, [bookAttached, bookFile, filePath, openReader, pagesFm, readerPosition, title]);

  const handleCloseReader = useCallback(() => {
    const progressPagePath = readerSessionRef.current?.progressPagePath;
    setReaderSession(null);
    if (progressPagePath) void flushBookReaderMeta(progressPagePath);
  }, []);

  const handleReaderPagesChange = useCallback((
    pages: string,
    extra?: { cfi: string; readPercent: number },
  ) => {
    const progressPagePath = readerSessionRef.current?.progressPagePath;
    if (!progressPagePath) return;
    void persistBookReaderMeta(progressPagePath, {
      pages,
      readerPosition: extra?.cfi,
      readPercent: extra?.readPercent,
    }).catch((error) => {
      console.error('Failed to persist book reader metadata', error);
    });
  }, []);

  const handleReaderQuote = useCallback((text: string, cfi: string) => {
    const quoteBookFile = readerSessionRef.current?.bookFile ?? bookFile;
    const refNumber = findReaderQuotes(Text.of(readText().split('\n'))).length + 1;
    void appendQuoteToDocument(filePath, formatBookQuote(text, cfi, quoteBookFile, refNumber))
      .catch((error) => console.error('Failed to append the book quote', error));
  }, [bookFile, filePath, readText]);

  useEffect(() => {
    if (bookAttached) warmReaderEngine();
  }, [bookAttached]);

  return {
    readerSession,
    handleOpenExternalUrl,
    handleReadBookCallout,
    handleReadBook,
    handleCloseReader,
    handleReaderPagesChange,
    handleReaderQuote,
  };
}
