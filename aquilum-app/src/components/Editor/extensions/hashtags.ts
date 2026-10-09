import { RangeSetBuilder, StateEffect } from '@codemirror/state';
import { Decoration, EditorView } from '@codemirror/view';
import { previewCaret } from './livePreviewConfig';
import { coloredTagsFacet, tagColor } from './plugins/coloredTags';
import { shouldRevealSyntax } from './livePreviewVisibility';
import { viewportCollector, type ViewportCollector } from './viewportScan';

const hashtagDecoration = Decoration.mark({
  class: 'q-cm-hashtag',
  tagName: 'span',
});

const HASHTAG_RE = /(?:^|\s)(#[a-zA-Zа-яА-Я0-9_]+(?:\/[a-zA-Zа-яА-Я0-9_]+)*)(?=\s|$)/g;

type Accumulator = {
  builder: RangeSetBuilder<Decoration>;
};

export const hashtagCollector: ViewportCollector<Accumulator> = {
  id: 'hashtags',
  scan: 'lines',
  triggers: { doc: true, selection: true, viewport: true, effects: [StateEffect.reconfigure] },

  begin: () => ({ builder: new RangeSetBuilder<Decoration>() }),

  enterLine: (line, { builder }, { state }) => {
    HASHTAG_RE.lastIndex = 0;
    let match = HASHTAG_RE.exec(line.text);
    while (match !== null) {
      const leadingSpace = match[0].length - match[1].length;
      const from = line.from + match.index + leadingSpace;
      const to = from + match[1].length;
      if (!shouldRevealSyntax(state.doc, previewCaret(state), from, to)) {
        const settings = state.facet(coloredTagsFacet);
        builder.add(from, to, settings ? Decoration.mark({
          class: 'q-cm-hashtag q-cm-hashtag--colored',
          tagName: 'span',
          attributes: { style: `--q-tag-color: ${tagColor(match[1].slice(1), settings)}` },
        }) : hashtagDecoration);
      }
      match = HASHTAG_RE.exec(line.text);
    }
  },

  finish: ({ builder }) => builder.finish(),
};

export const hashtagExtension = [
  viewportCollector(hashtagCollector),

  EditorView.theme({
    '.q-cm-hashtag': {
      display: 'inline-flex',
      justifyContent: 'center',
      alignItems: 'center',
      gap: 'var(--q-size-10)',
      padding: '0 var(--q-padding-sm)',
      lineHeight: 'var(--q-size-28)',
      textIndent: '0',
      borderRadius: 'var(--q-rounded-full)',
      background: 'color-mix(in srgb, var(--q-bg-accent) 10%, transparent)',
      color: 'var(--q-text-accent)',
      fontFamily: 'inherit',
      fontSize: 'inherit',
      fontWeight: 'inherit',
      whiteSpace: 'nowrap',
      userSelect: 'none',
    },
  }),
];
