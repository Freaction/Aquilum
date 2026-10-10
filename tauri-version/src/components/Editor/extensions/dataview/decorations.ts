import { type EditorState, type Range } from '@codemirror/state';
import { Decoration } from '@codemirror/view';
import type { LivePreviewConfig } from '../livePreviewConfig';
import { collectPreviewReplaceDecorations } from '../livePreviewEditableSpans';
import { findDataviewBlocks } from './constructs';
import { DataviewWidget } from './widget';

export function collectDataviewDecorations(
  state: EditorState,
  config: LivePreviewConfig | null,
): Range<Decoration>[] {
  if (!config) return [];
  const notePath = config.notePath();

  return collectPreviewReplaceDecorations(
    state,
    findDataviewBlocks(state).filter((block) => block.query.length > 0),
    (block) => new DataviewWidget({ query: block.query, notePath }),
  );
}
