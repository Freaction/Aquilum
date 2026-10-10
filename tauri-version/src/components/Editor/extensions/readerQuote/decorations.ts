import { type EditorState, type Range } from '@codemirror/state';
import { Decoration } from '@codemirror/view';
import type { LivePreviewConfig } from '../livePreviewConfig';
import { collectPreviewReplaceDecorations } from '../livePreviewEditableSpans';
import { findReaderQuotes } from './constructs';
import { ReaderQuoteWidget } from './widget';

export function collectReaderQuoteDecorations(
  state: EditorState,
  config: LivePreviewConfig | null,
): Range<Decoration>[] {
  if (!config) return [];

  return collectPreviewReplaceDecorations(
    state,
    findReaderQuotes(state.doc),
    (span, index) => new ReaderQuoteWidget({
      from: span.from,
      to: span.to,
      quoteText: span.quoteText,
      href: span.href,
      refLabel: span.refLabel ?? String(index + 1),
    }),
  );
}
