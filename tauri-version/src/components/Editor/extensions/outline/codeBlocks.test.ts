// @vitest-environment happy-dom
import { EditorState } from '@codemirror/state';
import { EditorView } from '@codemirror/view';
import { expect, it } from 'vitest';
import { editorMarkdownSupport } from '../markdownConfig';
import { codeBlockExtension } from '../codeBlock';
import { outlinePreview } from './preview';
import { listCalloutsExtension } from './listCallouts';

it.each(['```', '~~~'])('keeps list and callout preview out of %s fences, including after edits', fence => {
  const doc = `${fence}\n- text\n- ! important\n- [ ] task\n${fence}\n\n- outside`;
  const view = new EditorView({ parent: document.body, state: EditorState.create({
    doc, extensions: [editorMarkdownSupport, codeBlockExtension, outlinePreview, listCalloutsExtension(true)],
  }) });
  try {
    const check = () => {
      for (const line of view.dom.querySelectorAll('.q-md-code-line')) {
        expect(line.querySelector('.q-md-outline-bullet, .q-md-list-callout-marker, input')).toBeNull();
        expect(line.classList.contains('q-md-list-item')).toBe(false);
        expect(line.classList.contains('q-md-list-callout')).toBe(false);
      }
      expect(view.dom.querySelectorAll('.q-md-outline-bullet')).toHaveLength(1);
    };
    check();
    view.dispatch({ changes: { from: fence.length + 1, insert: '- added\n' } });
    check();
  } finally { view.destroy(); }
});
