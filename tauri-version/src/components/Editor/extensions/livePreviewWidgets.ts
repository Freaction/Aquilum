import { type EditorState, type Extension, type Range, StateField } from '@codemirror/state';
import { syntaxTree } from '@codemirror/language';
import { Decoration, type DecorationSet, EditorView } from '@codemirror/view';
import { collectBookCalloutDecorations } from './bookCallout/decorations';
import { bookCalloutDataReady, bookCalloutPrefetchExtension } from './bookCallout/prefetch';
import { collectDataviewDecorations } from './dataview/decorations';
import { editorFocusExtension, setEditorFocus } from './editorFocus';
import { dataviewDataReady, dataviewPrefetchExtension } from './dataview/prefetch';
import { collectImageEmbedDecorations } from './image/decorations';
import { setImageSource } from './image/focus';
import { setFrontmatterExpanded } from './frontmatterUi';
import { livePreviewConfigFacet, revealAtCaretFacet } from './livePreviewConfig';
import { collectReaderQuoteDecorations } from './readerQuote/decorations';

function collectLivePreviewWidgetDecorations(
  state: EditorState,
): Range<Decoration>[] {
  const config = state.facet(livePreviewConfigFacet);
  if (!config) return [];
  return [
    ...collectBookCalloutDecorations(state, config),
    ...collectDataviewDecorations(state, config),
    ...collectReaderQuoteDecorations(state, config),
    ...collectImageEmbedDecorations(state, config),
  ];
}

function buildLivePreviewWidgetDecorations(state: EditorState): DecorationSet {
  const ranges = collectLivePreviewWidgetDecorations(state);
  return ranges.length ? Decoration.set(ranges, true) : Decoration.none;
}

const livePreviewWidgetField = StateField.define<DecorationSet>({
  create: (state) => buildLivePreviewWidgetDecorations(state),
  update: (value, tr) => {
    if (
      tr.state.facet(revealAtCaretFacet) !== tr.startState.facet(revealAtCaretFacet)
      || tr.docChanged
      || tr.selection
      || syntaxTree(tr.state) !== syntaxTree(tr.startState)
      || tr.effects.some((effect) => effect.is(setFrontmatterExpanded)
        || effect.is(setImageSource)
        || effect.is(bookCalloutDataReady)
        || effect.is(dataviewDataReady)
        || effect.is(setEditorFocus))
    ) {
      return buildLivePreviewWidgetDecorations(tr.state);
    }
    return value;
  },
  provide: (field) => EditorView.decorations.from(field),
});

export function livePreviewWidgetExtension(): Extension {
  return [
    editorFocusExtension(),
    livePreviewWidgetField,
    EditorView.atomicRanges.of((view) => view.state.field(livePreviewWidgetField)),
    bookCalloutPrefetchExtension(),
    dataviewPrefetchExtension(),
  ];
}
