import {
  Decoration,
  type DecorationSet,
  EditorView,
  ViewPlugin,
  type ViewUpdate,
} from '@codemirror/view';
import { RangeSetBuilder } from '@codemirror/state';
import { getWikiHover, subscribeWikiHover } from '../../../modules/wikixiv';
import { wikiTermMatcher } from '../../../modules/wikixiv/termMatcher';

const highlightMark = Decoration.mark({ class: 'q-wiki-hover-highlight' });

function titleDecorations(view: EditorView): DecorationSet {
  if (!getWikiHover().filename) {
    return Decoration.none;
  }
  const len = view.state.doc.length;
  if (len === 0) return Decoration.none;
  return Decoration.set([highlightMark.range(0, len)]);
}

function calloutDecorations(view: EditorView): DecorationSet {
  const matcher = wikiTermMatcher(getWikiHover().terms);
  if (!matcher) return Decoration.none;

  const builder = new RangeSetBuilder<Decoration>();
  const text = view.state.doc.toString();
  for (const match of text.matchAll(/>\s*\[!(?:book|quote)\][^\n]*?\[\[\s*([^\]|#]+)(?:[|#][^\]]*)?\]\]/giu)) {
    const title = match[1].trim();
    matcher.lastIndex = 0;
    if (!matcher.test(title)) continue;
    const titleOffset = match[0].indexOf(match[1]);
    const from = match.index + titleOffset;
    builder.add(from, from + match[1].length, highlightMark);
  }
  return builder.finish();
}

function bodyDecorations(view: EditorView): DecorationSet {
  const matcher = wikiTermMatcher(getWikiHover().terms);
  if (!matcher) return Decoration.none;

  const builder = new RangeSetBuilder<Decoration>();
  for (const { from, to } of view.visibleRanges) {
    const text = view.state.doc.sliceString(from, to);
    matcher.lastIndex = 0;
    let match: RegExpExecArray | null;
    while ((match = matcher.exec(text)) !== null) {
      const start = from + match.index;
      builder.add(start, start + match[0].length, highlightMark);
      if (match.index === matcher.lastIndex) matcher.lastIndex += 1;
    }
  }
  return builder.finish();
}

function hoverPlugin(build: (view: EditorView) => DecorationSet) {
  return ViewPlugin.fromClass(
    class {
      decorations: DecorationSet;
      private readonly unsubscribe: () => void;

      constructor(view: EditorView) {
        this.decorations = build(view);
        this.unsubscribe = subscribeWikiHover(() => {
          this.decorations = build(view);
          view.dispatch({});
        });
      }

      update(update: ViewUpdate) {
        if (update.docChanged || update.viewportChanged) {
          this.decorations = build(update.view);
        }
      }

      destroy() {
        this.unsubscribe();
      }
    },
    { decorations: (plugin) => plugin.decorations },
  );
}

export const wikiHoverHighlightTitle = hoverPlugin(titleDecorations);
export const wikiHoverHighlight = [hoverPlugin(bodyDecorations), hoverPlugin(calloutDecorations)];
