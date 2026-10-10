// @vitest-environment happy-dom
import { act } from 'preact/test-utils';
import { afterEach, expect, it } from 'vitest';
import { EditorState, StateEffect, type Extension } from '@codemirror/state';
import { EditorView } from '@codemirror/view';
import { useEditorExtensions } from '../index';
import { DEFAULT_PLUGIN_SETTINGS } from '../../../../modules/settings';
import { setReadingMode } from '../../../../plugins/editor/readingMode';
import { coloredTagsFacet } from './coloredTags';
import { cursorTrailExtension } from './cursorTrail';
import { mountDom, type MountedDom } from '../../../../testing/mountDom';

let mounted: MountedDom;
let view: EditorView;
afterEach(() => { view?.destroy(); act(() => mounted?.unmount()); setReadingMode(false); });
const resolve = async (targets: string[]) => ({ paths: targets.map(() => null), complete: true });
const noop = () => {};
it('omits disabled plugins and reconfigures colors and reading mode when flags change', () => {
  let plugins = structuredClone(DEFAULT_PLUGIN_SETTINGS);
  let extensions: Extension = [];
  function Harness() {
    extensions = useEditorExtensions(resolve, noop, noop, true, true, true, null, undefined, undefined, true, 2, plugins);
    return null;
  }
  setReadingMode(true);
  act(() => { mounted = mountDom(<Harness />); });
  view = new EditorView({ parent: document.body, state: EditorState.create({ doc: '\n#project', extensions }) });
  expect(view.state.facet(coloredTagsFacet)).toBeNull();
  expect(view.plugin(cursorTrailExtension)).toBeNull();
  expect(view.state.readOnly).toBe(false);
  plugins = { ...plugins, coloredTags: { enabled: true, mixNested: true, tagColors: { project: 'blue' } }, cursorTrail: { enabled: true }, readingMode: { enabled: true, shortcut: null } };
  act(() => { mounted.update(<Harness />); });
  view.dispatch({ effects: StateEffect.reconfigure.of(extensions) });
  expect(view.state.readOnly).toBe(true);
  expect(view.plugin(cursorTrailExtension)).not.toBeNull();
  expect(view.dom.querySelector('.q-cm-hashtag')!.getAttribute('style')).toContain('var(--q-blue-500)');
  plugins = { ...plugins, coloredTags: { ...plugins.coloredTags, tagColors: { project: 'red' } } };
  act(() => { mounted.update(<Harness />); });
  view.dispatch({ effects: StateEffect.reconfigure.of(extensions) });
  expect(view.dom.querySelector('.q-cm-hashtag')!.getAttribute('style')).toContain('var(--q-red-500)');
  plugins = structuredClone(DEFAULT_PLUGIN_SETTINGS);
  act(() => { mounted.update(<Harness />); });
  view.dispatch({ effects: StateEffect.reconfigure.of(extensions) });
  expect(view.state.readOnly).toBe(false);
  expect(view.state.facet(EditorView.editable)).toBe(true);
  expect(view.state.facet(coloredTagsFacet)).toBeNull();
  expect(view.plugin(cursorTrailExtension)).toBeNull();
  expect(view.dom.querySelector('.q-cm-hashtag')!.getAttribute('style')).toBeNull();
});
