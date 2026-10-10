// @vitest-environment happy-dom
import { EditorState } from '@codemirror/state';
import { EditorView } from '@codemirror/view';
import { afterEach, expect, it, vi } from 'vitest';
import { TableWidget } from './widget';
import { setWrapperText } from './widgetDom';
import { findTablesInDoc } from './constructs';
import { WidgetSession } from './widgetSession';
import { livePreviewConfigFacet } from '../livePreviewConfig';

let view: EditorView;
let root: HTMLElement;
let session: WidgetSession | undefined;
afterEach(() => { session?.dispose(); session = undefined; if (root) new TableWidget({ from: 0, contentTo: 0, blockTo: 0, model: {} as never, text: '' }).destroy(root); view?.destroy(); root?.remove(); });

it('renders mixed Markdown and wiki links as text without buttons or icons', () => {
  const wrapper = document.createElement('div');
  setWrapperText(wrapper, 'before [site](https://example.com/a_(b)) and [[Note\\|alias]] after `[literal](url)`');
  expect(wrapper.textContent).toBe('before site and alias after [literal](url)');
  expect([...wrapper.querySelectorAll('a')].map(link => link.textContent)).toEqual(['site', 'alias']);
  expect(wrapper.querySelector('button, svg')).toBeNull();
  setWrapperText(wrapper, '`a\\|b`');
  expect(wrapper.textContent).toBe('a|b');
  expect(wrapper.querySelector('.q-md-code')).not.toBeNull();
  setWrapperText(wrapper, '[[Note]]');
  expect(wrapper.textContent).toBe('Note');
});

it('opens links through the host handlers and shows source while editing', () => {
  const doc = '| link |\n| --- |\n| [site](https://example.com) [[Note\\|alias]] |';
  const onOpenWikiLink = vi.fn();
  const onOpenExternalUrl = vi.fn();
  view = new EditorView({ parent: document.body, state: EditorState.create({ doc, extensions: [
    livePreviewConfigFacet.of({ workspacePath: null, notePath: () => '', resolveWikiLinks: async () => ({ paths: [], complete: true }), onReadBook: vi.fn(), onOpenWikiLink, onOpenExternalUrl }),
  ] }) });
  const table = findTablesInDoc(view.state.doc)[0];
  const widget = new TableWidget({ from: 0, contentTo: table.contentTo, blockTo: table.to, model: table.model, text: doc });
  root = widget.toDOM(view); document.body.append(root);
  const links = [...root.querySelectorAll<HTMLAnchorElement>('a')];
  expect(links.map(link => link.textContent)).toEqual(['site', 'alias']);
  links[0].dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true }));
  expect(root.querySelector('.q-md-table-cell-wrapper .cm-editor')).toBeNull();
  links[0].click();
  links[1].dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true, metaKey: true }));
  expect(onOpenExternalUrl).toHaveBeenCalledWith('https://example.com');
  expect(onOpenWikiLink).toHaveBeenCalledWith('Note', 'new-tab');
  widget.destroy(root);
  session = new WidgetSession(view, { from: 0, contentTo: table.contentTo, blockTo: table.to }, table.model, root);
  session.openCellEditor({ row: 1, col: 0 });
  const nested = EditorView.findFromDOM(root.querySelector('.cm-editor')!)!;
  expect(nested.state.doc.toString()).toBe('[site](https://example.com) [[Note\\|alias]]');
});
