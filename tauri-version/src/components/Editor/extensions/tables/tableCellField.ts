import { EditorSelection } from '@codemirror/state';
import { EditorView } from '@codemirror/view';
import { normalizeSingleLineText } from '../../../Common/codeMirror';
import { buildTableCellFieldExtensions } from './tableCellFieldExtensions';

type TableCellFieldNav = 'next' | 'prev' | 'down' | 'escape';

export interface TableCellFieldHandlers {
    onNavigate: (action: TableCellFieldNav) => void;
    onBlur: () => void;
}

interface TableCellFieldFocusOptions {
    clientX?: number;
    clientY?: number;
}

export class TableCellField {
    private view: EditorView | null = null;
    private detachedHost: HTMLElement | null = null;
    handlers: TableCellFieldHandlers | null = null;
    allowBlurExit = false;
    destroyed = false;

    constructor(
        private readonly hostView: EditorView,
        private readonly onHostFocusChange: (focused: boolean) => void,
    ) {}

    get value(): string {
        if (!this.view) return '';
        return normalizeSingleLineText(this.view.state.doc.toString());
    }

    get isAttached(): boolean {
        if (!this.view || !this.detachedHost) return false;
        return this.view.dom.parentElement !== this.detachedHost;
    }

    containsFocus(): boolean {
        return this.view?.hasFocus ?? false;
    }

    hasTextSelection(): boolean {
        if (!this.view) return false;
        return !this.view.state.selection.main.empty;
    }

    ensureCreated(detachedHost: HTMLElement): void {
        if (this.view) {
            this.detachedHost = detachedHost;
            return;
        }

        this.detachedHost = detachedHost;
        this.destroyed = false;

        this.view = new EditorView({
            doc: '',
            parent: detachedHost,
            extensions: buildTableCellFieldExtensions(this, this.hostView, this.onHostFocusChange),
        });

        this.view.dom.classList.add('q-md-table-cell-field');
    }

    attach(
        wrapper: HTMLElement,
        initial: string,
        handlers: TableCellFieldHandlers,
        options: TableCellFieldFocusOptions = {},
    ): void {
        if (!this.view || this.destroyed) return;

        this.handlers = handlers;
        this.allowBlurExit = false;
        wrapper.replaceChildren();
        wrapper.appendChild(this.view.dom);

        const next = normalizeSingleLineText(initial);
        const current = this.view.state.doc.toString();
        if (current !== next) {
            this.view.dispatch({
                changes: { from: 0, to: this.view.state.doc.length, insert: next },
            });
        }

        this.focusAt(options);
    }

    placeCaretAtEnd(): void {
        const view = this.view;
        if (!view || this.destroyed) return;
        view.dispatch({
            selection: EditorSelection.cursor(view.state.doc.length),
            scrollIntoView: false,
        });
    }

    detach(): string {
        const value = this.value;
        const view = this.view;
        if (view) {
            view.dispatch({
                selection: EditorSelection.cursor(view.state.selection.main.head),
                scrollIntoView: false,
            });
        }
        if (this.view && this.detachedHost) {
            this.detachedHost.appendChild(this.view.dom);
        }
        this.allowBlurExit = false;
        this.handlers = null;
        return value;
    }

    focusAt(options: TableCellFieldFocusOptions = {}): void {
        const view = this.view;
        if (!view || this.destroyed) return;

        this.allowBlurExit = false;
        view.focus();
        if (options.clientX != null && options.clientY != null) {
            this.placeCaretAt(options);
        }

        requestAnimationFrame(() => {
            this.allowBlurExit = true;
        });
    }

    placeCaretAt(options: TableCellFieldFocusOptions): void {
        const view = this.view;
        if (!view || this.destroyed) return;

        let pos = view.state.doc.length;
        if (options.clientX != null && options.clientY != null) {
            const at = view.posAtCoords({ x: options.clientX, y: options.clientY });
            if (at != null) pos = at;
        }

        view.dispatch({
            selection: EditorSelection.cursor(pos),
            scrollIntoView: false,
        });
    }

    selectFromCoords(anchor: { clientX: number; clientY: number }, focus: { clientX: number; clientY: number }): void {
        const view = this.view;
        if (!view || this.destroyed) return;
        const from = view.posAtCoords({ x: anchor.clientX, y: anchor.clientY });
        const to = view.posAtCoords({ x: focus.clientX, y: focus.clientY });
        if (from == null || to == null) return;
        view.dispatch({
            selection: EditorSelection.range(from, to),
            scrollIntoView: false,
        });
    }

    destroy(): void {
        this.destroyed = true;
        this.allowBlurExit = false;
        this.handlers = null;
        if (this.view) {
            this.view.destroy();
            this.view = null;
        }
        this.detachedHost = null;
    }
}
