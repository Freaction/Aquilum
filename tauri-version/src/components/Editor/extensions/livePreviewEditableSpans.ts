import type { EditorState, Range, Text } from '@codemirror/state';
import { Decoration, WidgetType } from '@codemirror/view';
import { revealAtCaretFacet } from './livePreviewConfig';
import { editorHasFocus } from './editorFocus';
import { findBookCallouts } from './bookCallout/constructs';
import { findReaderQuotes, readerQuoteEditEntryPos } from './readerQuote/constructs';

type EditablePreviewSpan = {
  from: number;
  to: number;
};

export function collectEditablePreviewSpans(doc: Text): EditablePreviewSpan[] {
  return [...findBookCallouts(doc), ...findReaderQuotes(doc)];
}

export function selectionIntersectsEditablePreview(
  state: EditorState,
  span: EditablePreviewSpan,
): boolean {
  if (!state.facet(revealAtCaretFacet)) return false;
  const sel = state.selection.main;
  return sel.from <= span.to && sel.to >= span.from;
}

export function selectionInEditablePreviewBlock(state: EditorState): boolean {
  for (const span of collectEditablePreviewSpans(state.doc)) {
    if (selectionIntersectsEditablePreview(state, span)) return true;
  }
  return false;
}

const PREVIEW_SPAN_LINK_MARKERS = new Set(['LinkMark', 'URL']);

export function shouldRevealEditablePreviewMarker(
  spans: readonly EditablePreviewSpan[],
  head: number,
  markFrom: number,
  markerName?: string,
): boolean {
  if (markerName && PREVIEW_SPAN_LINK_MARKERS.has(markerName)) return false;
  for (const span of spans) {
    if (markFrom < span.from || markFrom > span.to) continue;
    if (head >= span.from && head <= span.to) return true;
  }
  return false;
}

export function editablePreviewBlockEntryPos(
  state: EditorState,
  caretLine: number,
  forward: boolean,
): number | null {
  let best: { pos: number; line: number } | null = null;

  for (const span of collectEditablePreviewSpans(state.doc)) {
    if (selectionIntersectsEditablePreview(state, span)) continue;
    const start = state.doc.lineAt(span.from).number;
    const end = state.doc.lineAt(span.to).number;

    let pos: number | null = null;
    if (forward && caretLine === start - 1) {
      pos = span.from;
    } else if (!forward && caretLine === end + 1) {
      const quote = findReaderQuotes(state.doc).find(
        (candidate) => candidate.from === span.from && candidate.to === span.to,
      );
      pos = quote ? readerQuoteEditEntryPos(quote) : span.to;
    }
    if (pos == null) continue;

    const line = forward ? start : end;
    if (!best || (forward ? line < best.line : line > best.line)) {
      best = { pos, line };
    }
  }

  return best?.pos ?? null;
}

export function collectPreviewReplaceDecorations<T extends { from: number; to: number }>(
  state: EditorState,
  spans: readonly T[],
  createWidget: (span: T, index: number) => WidgetType,
  isEditing: (state: EditorState, span: T) => boolean = selectionIntersectsEditablePreview,
): Range<Decoration>[] {
  if (spans.length === 0) return [];

  const docLen = state.doc.length;
  const ranges: Range<Decoration>[] = [];

  for (let i = 0; i < spans.length; i += 1) {
    const span = spans[i]!;
    if (span.from < 0 || span.to > docLen || span.from >= span.to) continue;
    if (state.facet(revealAtCaretFacet) && editorHasFocus(state) && isEditing(state, span)) continue;

    ranges.push(
      Decoration.replace({
        widget: createWidget(span, i),
        block: true,
      }).range(span.from, span.to),
    );
  }

  return ranges;
}
