import { revealAtCaretFacet } from './livePreviewConfig';
import type { EditorState } from '@codemirror/state';
import { Decoration, EditorView } from '@codemirror/view';
import type { Tree } from '@lezer/common';
import { calloutMarkerRange, isBookCalloutHeader } from './blockquoteScan';
import { findReaderQuotes } from './readerQuote/constructs';
import { selectionIntersectsEditablePreview } from './livePreviewEditableSpans';

const MARKER_WIDTH = '2ch';

const blockquoteLines = {
  alone: Decoration.line({ class: 'q-md-blockquote' }),
  runStart: Decoration.line({ class: 'q-md-blockquote q-md-blockquote--joined-down' }),
  runMiddle: Decoration.line({
    class: 'q-md-blockquote q-md-blockquote--joined-up q-md-blockquote--joined-down',
  }),
  runEnd: Decoration.line({ class: 'q-md-blockquote q-md-blockquote--joined-up' }),
};

const calloutMarker = Decoration.mark({ class: 'q-md-syntax-char' });

export const blockquoteStyles = {
  '.q-md-blockquote': {
    position: 'relative',
    boxSizing: 'border-box',
    '--q-md-quote-indent': `calc(var(--q-caret-width) + ${MARKER_WIDTH})`,
    paddingLeft: 'var(--q-md-quote-indent)',
  },
  '.cm-line.q-md-blockquote': {
    paddingLeft: 'var(--q-md-quote-indent)',
    textIndent: `-${MARKER_WIDTH}`,
  },
  '.q-md-blockquote::before': {
    content: '""',
    position: 'absolute',
    left: '0',
    top: 'var(--q-unit-2)',
    bottom: 'var(--q-unit-2)',
    width: 'var(--q-caret-width)',
    maxWidth: 'var(--q-caret-width)',
    borderRadius: 'var(--q-radius-full)',
    backgroundColor: 'var(--q-bg-accent)',
    pointerEvents: 'none',
  },
  '.q-md-blockquote.q-md-blockquote--joined-up::before': {
    top: '0',
    borderTopLeftRadius: '0',
    borderTopRightRadius: '0',
  },
  '.q-md-blockquote.q-md-blockquote--joined-down::before': {
    bottom: '0',
    borderBottomLeftRadius: '0',
    borderBottomRightRadius: '0',
  },
};

export const blockquoteTheme = EditorView.theme(blockquoteStyles);

export function collectBlockquoteDecorations(
  state: EditorState,
  tree: Tree,
  ranges: readonly { from: number; to: number }[],
): { from: number; to: number; dec: Decoration }[] {
  const doc = state.doc;
  const readerQuotes = findReaderQuotes(doc);
  const sel = state.selection.main;
  const out: { from: number; to: number; dec: Decoration }[] = [];

  for (const range of ranges) {
    tree.iterate({
      from: range.from,
      to: Math.min(range.to, tree.length),
      enter(node) {
        if (node.name !== 'Blockquote') return;
        const isBookCallout = isBookCalloutHeader(doc.lineAt(node.from).text);

        const startLine = doc.lineAt(node.from).number;
        let endLine = doc.lineAt(Math.min(node.to, doc.length)).number;
        while (endLine > startLine && !/^>/.test(doc.line(endLine).text)) endLine -= 1;
        const focused = state.facet(revealAtCaretFacet) && sel.from <= doc.line(endLine).to && sel.to >= doc.line(startLine).from;

        const barLines: number[] = [];
        for (let lineNo = startLine; lineNo <= endLine; lineNo += 1) {
          const line = doc.line(lineNo);
          if (!/^>/.test(line.text)) continue;

          const marker = focused ? calloutMarkerRange(line.text) : null;
          if (marker) {
            out.push({
              from: line.from + marker.from,
              to: line.from + marker.to,
              dec: calloutMarker,
            });
          }

          if (isBookCallout) continue;
          const widget = readerQuotes.find(
            (span) => !(span.to < line.from || span.from > line.to)
              && !selectionIntersectsEditablePreview(state, span),
          );
          if (widget) continue;
          barLines.push(lineNo);
        }

        for (let i = 0; i < barLines.length; i += 1) {
          const lineNo = barLines[i];
          const joinedUp = barLines[i - 1] === lineNo - 1;
          const joinedDown = barLines[i + 1] === lineNo + 1;
          const dec = joinedUp && joinedDown ? blockquoteLines.runMiddle
            : joinedDown ? blockquoteLines.runStart
            : joinedUp ? blockquoteLines.runEnd
            : blockquoteLines.alone;
          const from = doc.line(lineNo).from;
          out.push({ from, to: from, dec });
        }
        return false;
      },
    });
  }

  out.sort((a, b) => a.from - b.from || a.to - b.to);
  return out;
}
