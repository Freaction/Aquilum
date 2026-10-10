import type { EditorView } from '@codemirror/view';

export function setTableInteractionChrome(view: EditorView, active: boolean): void {
    view.dom.classList.toggle('q-md-table-interaction', active);
}
