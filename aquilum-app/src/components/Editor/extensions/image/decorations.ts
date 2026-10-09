import { type EditorState, type Range } from '@codemirror/state';
import { Decoration } from '@codemirror/view';
import { resolveVaultAssetUrl } from '../../../../modules/docs/vaultAssets';
import { cachedAttachmentUrl } from '../../../../modules/docs/vaultAttachments';
import { revealAtCaretFacet, type LivePreviewConfig } from '../livePreviewConfig';
import { findImageEmbeds } from './constructs';
import { imageSourcePos } from './focus';
import { ImageEmbedWidget } from './widget';

export function collectImageEmbedDecorations(
  state: EditorState,
  config: LivePreviewConfig | null,
): Range<Decoration>[] {
  if (!config) return [];

  const source = state.facet(revealAtCaretFacet) ? imageSourcePos(state) : null;
  const docLength = state.doc.length;
  const ranges: Range<Decoration>[] = [];

  for (const span of findImageEmbeds(state.doc)) {
    if (span.from < 0 || span.to > docLength || span.from >= span.to) continue;

    const widget = new ImageEmbedWidget({
      url: span.wiki
        ? cachedAttachmentUrl(config.workspacePath, span.src) ?? ''
        : resolveVaultAssetUrl(config.workspacePath, span.src, ''),
      width: span.width,
      align: span.align,
      crop: span.crop,
      kind: span.kind,
      wikiTarget: span.wiki ? span.src : null,
      workspacePath: config.workspacePath,
    });

    ranges.push(Decoration.widget({ widget, block: true, side: -1 }).range(span.from));
    if (span.from !== source) {
      ranges.push(Decoration.replace({ block: true }).range(span.from, span.to));
    }
  }

  return ranges;
}
