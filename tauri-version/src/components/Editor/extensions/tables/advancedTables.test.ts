// @vitest-environment happy-dom
import { EditorSelection, EditorState } from '@codemirror/state';
import { drawSelection, EditorView } from '@codemirror/view';
import { act } from 'preact/test-utils';
import { afterEach, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { t } from '../../../../i18n';
import { advancedTablesConfig, transformTable } from './advancedTables';
import { findTablesInDoc } from './constructs';
import { openWidgetStructureMenu } from './widgetSessionMenu';
import { closeTableContextMenu } from './contextMenu';
import { renderTableWidgetDom } from './widgetDom';
import { WidgetSession } from './widgetSession';
import { syncStripSizes } from './geometry';
import { tablePreview } from './preview';
import { tableTheme } from './theme';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
const markdown = '| name | n |\n| --- | --: |\n| z | 10 |\n| a | 2 |';
const formatted = '| name | n   |\n| ---- | ---: |\n| z    | 10  |\n| a    | 2   |';
const actions = { onMerge: vi.fn(), onUnmerge: vi.fn(), onAlignColumn: vi.fn(), onDeleteRow: vi.fn(), onDeleteColumn: vi.fn(), onDeleteTable: vi.fn() };
let view: EditorView;
let session: WidgetSession | undefined;
let root: HTMLElement;
function create(options = { enabled: true, formatOnLeave: false }, readOnly = false, doc = markdown) {
  view = new EditorView({ parent: document.body, state: EditorState.create({ doc, extensions: [advancedTablesConfig.of(options), EditorState.readOnly.of(readOnly)] }) });
  const table = findTablesInDoc(view.state.doc)[0];
  root = renderTableWidgetDom(table.model);
  document.body.append(root);
  return table;
}
afterEach(async () => {
  await act(async () => { closeTableContextMenu(); });
  session?.dispose(); session = undefined;
  view?.destroy(); root?.remove();
  document.body.replaceChildren();
  vi.resetAllMocks();
});
const item = (key: string) => [...document.querySelectorAll<HTMLButtonElement>('[role="menuitem"]')].find(button => button.textContent === t(`plugins.advancedTables.${key}`));

it.each(['auto', 'explicit'] as const)('aligns %s addition bands to the cell grid rather than the strip chrome', (sizing) => {
  const table = create(undefined, false, markdown + (sizing === 'explicit' ? '\n<!--q-table:{"merges":[],"rows":[40,40,40],"cols":[180,140]}-->' : ''));
  const grid = root.querySelector<HTMLElement>('.q-md-table-editor')!;
  const scroll = root.querySelector<HTMLElement>('.q-md-table-scroll')!;
  const addRow = root.querySelector<HTMLElement>('.q-md-table-add-row')!;
  const addCol = root.querySelector<HTMLElement>('.q-md-table-add-col')!;
  vi.spyOn(grid, 'getBoundingClientRect').mockReturnValue(new DOMRect(40, 50, 320, 96));
  vi.spyOn(scroll, 'getBoundingClientRect').mockReturnValue(new DOMRect(10, 20, 350, 160));
  syncStripSizes(root, table.model);
  expect(addRow.parentElement).toBe(grid.parentElement);
  expect(addRow.style.width).toBe('320px');
  expect(addCol.style.height).toBe('96px');
  expect(addCol.style.marginTop).toBe('30px');
});

it('hides only an empty parent caret at table boundaries and restores ordinary text caret', () => {
  view = new EditorView({ parent: document.body, state: EditorState.create({
    doc: 'before\n\n' + markdown + '\n\nafter',
    extensions: [tablePreview, tableTheme, drawSelection(), EditorState.allowMultipleSelections.of(true)],
  }) });
  const table = findTablesInDoc(view.state.doc)[0];
  for (const head of [table.from, table.to]) {
    view.dispatch({ selection: { anchor: head } });
    expect(view.dom.classList.contains('q-md-table-boundary-caret')).toBe(true);
    expect(view.dom.classList.contains('q-md-table-interaction')).toBe(false);
    expect(getComputedStyle(view.scrollDOM.querySelector(':scope > .cm-cursorLayer')!).display).toBe('none');
    expect(getComputedStyle(view.scrollDOM.querySelector(':scope > .cm-selectionLayer')!).display).not.toBe('none');
    view.dispatch({ selection: { anchor: table.from, head: table.to } });
    expect(view.dom.classList.contains('q-md-table-boundary-caret')).toBe(false);
  }
  view.dispatch({ selection: EditorSelection.create([EditorSelection.cursor(2), EditorSelection.cursor(table.from)], 1) });
  expect(view.dom.classList.contains('q-md-table-boundary-caret')).toBe(false);
  view.dispatch({ selection: { anchor: 2 } });
  expect(view.dom.classList.contains('q-md-table-boundary-caret')).toBe(false);
  view.dispatch({ selection: { anchor: table.from } });
  view.dispatch({ changes: { from: table.from, to: table.contentTo, insert: 'ordinary text' } });
  expect(view.dom.classList.contains('q-md-table-boundary-caret')).toBe(false);
});
it('adds format and both sort menu items only when advanced actions are supplied', async () => {
  const table = create();
  await act(async () => { openWidgetStructureMenu(0, 0, { row: 1, col: 1 }, table.model, { canMerge: false, actions }); });
  expect(item('format')).toBeUndefined();
  const format = vi.fn(); const sort = vi.fn();
  const open = () => act(async () => { openWidgetStructureMenu(0, 0, { row: 1, col: 1 }, table.model, { canMerge: false, actions, advanced: { format, sort, readOnly: false } }); });
  await open(); await act(async () => { item('format')!.click(); });
  expect(format).toHaveBeenCalledOnce();
  await open(); await act(async () => { item('sortAscending')!.click(); });
  expect(sort).toHaveBeenLastCalledWith(false);
  await open(); await act(async () => { item('sortDescending')!.click(); });
  expect(sort).toHaveBeenLastCalledWith(true);
});
it('disables sorting for vertical merges but leaves formatting available', async () => {
  const table = create();
  const format = vi.fn(); const sort = vi.fn();
  table.model.merges = [{ top: 1, left: 0, bottom: 2, right: 0 }];
  await act(async () => { openWidgetStructureMenu(0, 0, { row: 1, col: 1 }, table.model, { canMerge: false, actions, advanced: { format, sort, readOnly: false } }); });
  expect(item('format')!.disabled).toBe(false);
  expect(item('sortAscending')!.disabled).toBe(true);
  expect(item('sortDescending')!.disabled).toBe(true);
  item('sortAscending')!.click(); expect(sort).not.toHaveBeenCalled();
});
it('disables all transformation items in reading mode', async () => {
  const table = create(undefined, true);
  await act(async () => { openWidgetStructureMenu(0, 0, { row: 1, col: 0 }, table.model, { canMerge: false, actions, advanced: { format: vi.fn(), sort: vi.fn(), readOnly: true } }); });
  for (const key of ['format', 'sortAscending', 'sortDescending']) expect(item(key)!.disabled).toBe(true);
  await transformTable(view, 0, root); expect(invoke).not.toHaveBeenCalled();
});
it('routes the widget menu to Rust using the current column', async () => {
  const table = create();
  session = new WidgetSession(view, { from: 0, contentTo: table.contentTo, blockTo: table.to }, table.model, root);
  vi.mocked(invoke).mockResolvedValue(formatted);
  const cell = root.querySelector<HTMLElement>('td[data-row="1"][data-col="1"]')!;
  cell.addEventListener('contextmenu', event => session!.onContextMenu(event));
  await act(async () => { cell.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true })); });
  await act(async () => { item('sortDescending')!.click(); });
  expect(invoke).toHaveBeenCalledWith('table_sort', { markdown, column: 1, descending: true });
  expect(view.state.doc.toString()).toBe(formatted);
});
it('formats on leaving the whole table, not on moving focus inside', async () => {
  const table = create({ enabled: true, formatOnLeave: true });
  session = new WidgetSession(view, { from: 0, contentTo: table.contentTo, blockTo: table.to }, table.model, root);
  vi.mocked(invoke).mockResolvedValue(formatted);
  const inside = document.createElement('button'); root.append(inside); inside.focus();
  root.dispatchEvent(new FocusEvent('focusout', { bubbles: true }));
  await Promise.resolve(); expect(invoke).not.toHaveBeenCalled();
  view.contentDOM.focus();
  await vi.waitFor(() => expect(invoke).toHaveBeenCalledWith('table_format', { markdown }));
  await vi.waitFor(() => expect(view.state.doc.toString()).toBe(formatted));
});
it('leaves auto-format off by default and when the plugin is disabled', async () => {
  for (const options of [{ enabled: true, formatOnLeave: false }, { enabled: false, formatOnLeave: true }]) {
    const table = create(options);
    session = new WidgetSession(view, { from: 0, contentTo: table.contentTo, blockTo: table.to }, table.model, root);
    root.dispatchEvent(new FocusEvent('focusout', { bubbles: true }));
    await Promise.resolve(); await Promise.resolve();
    expect(invoke).not.toHaveBeenCalled();
    session.dispose(); session = undefined; view.destroy(); root.remove();
  }
});
it('discards a late response after document edits', async () => {
  create();
  let resolve!: (value: string) => void;
  vi.mocked(invoke).mockImplementation(() => new Promise<string>(done => { resolve = done; }));
  const pending = transformTable(view, 0, root);
  view.dispatch({ changes: { from: view.state.doc.length, insert: '\nnew text' } });
  resolve(formatted); await pending;
  expect(view.state.doc.toString()).toBe(markdown + '\nnew text');
});
it('reports command failure without changing the document', async () => {
  create(); vi.mocked(invoke).mockRejectedValue({ code: 'invalid' });
  await transformTable(view, 0, root);
  expect(view.state.doc.toString()).toBe(markdown);
  expect(root.querySelector('[role="status"]')!.textContent).toBe(t('plugins.advancedTables.failed'));
});

it('keeps a new nested cell draft when an earlier command finishes', async () => {
  const table = create();
  session = new WidgetSession(view, { from: 0, contentTo: table.contentTo, blockTo: table.to }, table.model, root);
  let resolve!: (value: string) => void;
  vi.mocked(invoke).mockImplementation(() => new Promise<string>(done => { resolve = done; }));
  const pending = transformTable(view, 0, root);
  session.openCellEditor({ row: 1, col: 0 });
  const nested = EditorView.findFromDOM(root.querySelector('.cm-editor')!)!;
  nested.dispatch({ changes: { from: 0, to: nested.state.doc.length, insert: 'draft' } });
  resolve(formatted); await pending;
  expect(view.state.doc.toString()).toBe(markdown);
  expect(nested.state.doc.toString()).toBe('draft');
});
it('flushes the edited cell before formatting on leave', async () => {
  const table = create({ enabled: true, formatOnLeave: true });
  session = new WidgetSession(view, { from: 0, contentTo: table.contentTo, blockTo: table.to }, table.model, root);
  vi.mocked(invoke).mockImplementation((_command, args) => Promise.resolve((args as { markdown: string }).markdown));
  session.openCellEditor({ row: 1, col: 0 });
  const nested = EditorView.findFromDOM(root.querySelector('.cm-editor')!)!;
  nested.dispatch({ changes: { from: 0, to: nested.state.doc.length, insert: 'edited' } });
  view.contentDOM.focus();
  await vi.waitFor(() => expect(invoke).toHaveBeenCalled());
  expect(vi.mocked(invoke).mock.calls[0][1]).toEqual({ markdown: expect.stringContaining('edited') });
  expect(view.state.doc.toString()).toContain('edited');
});

it('formats the latest cell text when Escape leaves the table', async () => {
  const table = create({ enabled: true, formatOnLeave: true });
  session = new WidgetSession(view, { from: 0, contentTo: table.contentTo, blockTo: table.to }, table.model, root);
  vi.mocked(invoke).mockImplementation((_command, args) => Promise.resolve((args as { markdown: string }).markdown));
  session.openCellEditor({ row: 1, col: 0 });
  const nested = EditorView.findFromDOM(root.querySelector('.cm-editor')!)!;
  nested.dispatch({ changes: { from: 0, to: nested.state.doc.length, insert: 'escaped edit' } });
  nested.contentDOM.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
  await vi.waitFor(() => expect(invoke).toHaveBeenCalled());
  expect(vi.mocked(invoke).mock.calls[0][1]).toEqual({ markdown: expect.stringContaining('escaped edit') });
  expect(view.state.doc.toString()).toContain('escaped edit');
});
