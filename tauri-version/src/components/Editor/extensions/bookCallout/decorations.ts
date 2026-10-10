import { type EditorState, type Range } from '@codemirror/state';
import { Decoration } from '@codemirror/view';
import type { LivePreviewConfig } from '../livePreviewConfig';
import { collectPreviewReplaceDecorations } from '../livePreviewEditableSpans';
import { groupAdjacentBookCallouts } from './constructs';
import { BookCalloutGroupWidget } from './groupWidget';

export function collectBookCalloutDecorations(
  state: EditorState,
  config: LivePreviewConfig | null,
): Range<Decoration>[] {
  if (!config) return [];

  return collectPreviewReplaceDecorations(
    state,
    groupAdjacentBookCallouts(state.doc),
    (group) => new BookCalloutGroupWidget({
      group,
      workspacePath: config.workspacePath,
      resolveWikiLinks: config.resolveWikiLinks,
    }),
  );
}
