// @vitest-environment happy-dom
import { ChangeSet, EditorState } from '@codemirror/state';
import { EditorView } from '@codemirror/view';
import { collab, receiveUpdates } from '@codemirror/collab';
import { syntaxTree } from '@codemirror/language';
import { afterEach, expect, it } from 'vitest';
import { readingModeExtension } from './readingMode';
import { setReadingMode } from '../../../../plugins/editor/readingMode';
import { registerEditor, editorFor } from '../../openEditors';
import { editorMarkdownSupport } from '../markdownConfig';
import { collectCollapseRanges, livePreviewExtension } from '../livePreviewPlugin';

const views: EditorView[] = [];
const unregister: (() => void)[] = [];
function create(path: string) {
  const view = new EditorView({ parent: document.body, state: EditorState.create({
    doc: '[[note|label]]', selection: { anchor: 5 }, extensions: [editorMarkdownSupport, readingModeExtension()],
  }) });
  views.push(view);
  unregister.push(registerEditor(() => path, view));
  return view;
}
afterEach(() => { unregister.splice(0).forEach(fn => fn()); views.splice(0).forEach(view => view.destroy()); setReadingMode(false); });
it('switches all registered editors and newly opened tabs back and forth', () => {
  create('/one.md'); create('/two.md');
  expect(collectCollapseRanges(editorFor('/one.md')!.state, syntaxTree(editorFor('/one.md')!.state))).toEqual([]);
  setReadingMode(true);
  create('/three.md');
  for (const path of ['/one.md', '/two.md', '/three.md']) {
    const view = editorFor(path)!;
    expect(view.state.readOnly).toBe(true);
    expect(view.state.facet(EditorView.editable)).toBe(false);
    expect(collectCollapseRanges(view.state, syntaxTree(view.state)).length).toBeGreaterThan(0);
  }
  setReadingMode(false);
  for (const view of views) {
    expect(view.state.readOnly).toBe(false);
    expect(view.state.facet(EditorView.editable)).toBe(true);
    expect(collectCollapseRanges(view.state, syntaxTree(view.state))).toEqual([]);
  }
});
it('reads current mode when a prebuilt extension mounts later', () => {
  const extensions = readingModeExtension();
  setReadingMode(true);
  const view = new EditorView({ parent: document.body, state: EditorState.create({ doc: 'text', extensions }) });
  views.push(view);
  expect(view.state.readOnly).toBe(true);
  expect(view.state.facet(EditorView.editable)).toBe(false);
});

it('blocks widget changes while still accepting synchronized document updates', () => {
  const view = new EditorView({ parent: document.body, state: EditorState.create({
    doc: 'note', extensions: [collab({ clientID: 'local' }), readingModeExtension()],
  }) });
  views.push(view);
  setReadingMode(true);
  view.dispatch({ changes: { from: 0, insert: 'widget edit' } });
  expect(view.state.doc.toString()).toBe('note');
  view.dispatch(receiveUpdates(view.state, [{ clientID: 'disk', changes: ChangeSet.of({ from: 4, insert: ' updated' }, 4) }]));
  expect(view.state.doc.toString()).toBe('note updated');
  setReadingMode(false);
  view.dispatch({ changes: { from: 0, insert: 'edit ' } });
  expect(view.state.doc.toString()).toBe('edit note updated');
});

it('keeps the wiki source collapsed at the caret and restores it on toggle', () => {
  const view = new EditorView({ parent: document.body, state: EditorState.create({
    doc: '[[note|label]]', selection: { anchor: 5 }, extensions: [editorMarkdownSupport, readingModeExtension(), livePreviewExtension({
      resolveWikiLinks: async targets => ({ paths: targets.map(() => null), complete: true }),
      workspacePath: null, notePath: () => '', onOpenWikiLink: () => {}, onOpenExternalUrl: () => {}, onReadBook: () => {},
    })],
  }) });
  views.push(view);
  expect(view.contentDOM.textContent).toContain('[[note|label]]');
  setReadingMode(true);
  expect(view.contentDOM.textContent).toBe('label');
  view.dispatch({ selection: { anchor: 8 } });
  expect(view.contentDOM.textContent).toBe('label');
  setReadingMode(false);
  expect(view.contentDOM.textContent).toContain('[[note|label]]');
});
