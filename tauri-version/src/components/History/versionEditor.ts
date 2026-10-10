import { unifiedMergeView } from '@codemirror/merge';
import { EditorState } from '@codemirror/state';
import { drawSelection, EditorView } from '@codemirror/view';
import type { VersionTexts } from '../../modules/history';
import { aquilumCodeMirrorTheme } from '../Common/codeMirror';

const DIFF_TIMEOUT_MS = 300;

export function createVersionEditor(parent: HTMLElement, texts: VersionTexts): EditorView {
  const changes = texts.previous === null
    ? []
    : unifiedMergeView({
      original: texts.previous,
      mergeControls: false,
      gutter: false,
      syntaxHighlightDeletions: false,
      diffConfig: { timeout: DIFF_TIMEOUT_MS },
    });
  return new EditorView({
    parent,
    state: EditorState.create({
      doc: texts.text,
      extensions: [
        aquilumCodeMirrorTheme,
        drawSelection(),
        EditorView.lineWrapping,
        EditorState.readOnly.of(true),
        changes,
      ],
    }),
  });
}
