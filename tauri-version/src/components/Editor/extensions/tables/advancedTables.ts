import { Facet } from '@codemirror/state';
import type { EditorView } from '@codemirror/view';
import { invoke } from '@tauri-apps/api/core';
import type { PluginSettings } from '../../../../modules/settings';
import { t } from '../../../../i18n';
import { findTablesInDoc } from './constructs';

export const advancedTablesConfig = Facet.define<PluginSettings['advancedTables'], PluginSettings['advancedTables'] | null>({
  combine: values => values[values.length - 1] ?? null,
});
const requests = new WeakMap<EditorView, number>();

export async function transformTable(view: EditorView, from: number, root: HTMLElement, column?: number, descending = false): Promise<void> {
  if (view.state.readOnly || !view.state.facet(advancedTablesConfig)?.enabled) return;
  const table = findTablesInDoc(view.state.doc).find(table => table.from === from);
  if (!table) return;
  const doc = view.state.doc;
  const request = (requests.get(view) ?? 0) + 1;
  requests.set(view, request);
  const markdown = doc.sliceString(table.from, table.contentTo);
  try {
    const result = await invoke<string>(column === undefined ? 'table_format' : 'table_sort', {
      markdown, ...(column === undefined ? {} : { column, descending }),
    });
    // Поздний ответ не должен затирать редактирование или результат нового запроса.
    if (!view.dom.isConnected || !root.isConnected || view.state.doc !== doc || requests.get(view) !== request || view.state.readOnly || !view.state.facet(advancedTablesConfig)?.enabled) return;
    if (root.querySelector('td .cm-editor')) return;
    root.querySelector('.q-advanced-table-status')?.remove();
    if (result !== markdown) view.dispatch({ changes: { from: table.from, to: table.contentTo, insert: result }, userEvent: 'input' });
  } catch {
    if (!root.isConnected || requests.get(view) !== request) return;
    let status = root.querySelector<HTMLElement>('.q-advanced-table-status');
    if (!status) {
      status = document.createElement('span');
      status.className = 'q-advanced-table-status';
      status.setAttribute('role', 'status');
      status.setAttribute('aria-live', 'polite');
      root.append(status);
    }
    status.textContent = t('plugins.advancedTables.failed');
  }
}
