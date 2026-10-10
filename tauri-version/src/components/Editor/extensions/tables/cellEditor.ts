import type { EditorView } from '@codemirror/view';
import type { CellRef } from './model';
import { TableCellField } from './tableCellField';
import { setWrapperText } from './widgetDom';

export type CellNavAction = 'next' | 'prev' | 'down' | 'escape' | 'blur';

interface CellEditorCallbacks {
    onDone: (action: CellNavAction, value: string) => void;
}

export interface CellFlush {
    cell: CellRef;
    value: string;
}

interface CellActivateOptions {
    clientX?: number;
    clientY?: number;
}

export class TableCellEditor {
    private readonly field: TableCellField;
    private wrapper: HTMLElement | null = null;
    private cell: CellRef | null = null;
    private callbacks: CellEditorCallbacks | null = null;

    constructor(
        hostView: EditorView,
        detachedHost: HTMLElement,
        onFocusChange: (focused: boolean) => void,
    ) {
        this.field = new TableCellField(hostView, onFocusChange);
        this.field.ensureCreated(detachedHost);
    }

    get activeCell(): CellRef | null {
        return this.field.isAttached ? this.cell : null;
    }

    get activeField(): TableCellField {
        return this.field;
    }

    destroy(): void {
        if (this.field.isAttached) this.releaseWrapper(this.field.detach());
        this.field.destroy();
    }

    flush(): CellFlush | null {
        if (!this.cell || !this.wrapper || !this.field.isAttached) return null;
        const cell = this.cell;
        const value = this.field.detach();
        this.releaseWrapper(value);
        return { cell, value };
    }

    activate(
        wrapper: HTMLElement,
        cell: CellRef,
        initial: string,
        callbacks: CellEditorCallbacks,
        options: CellActivateOptions = {},
    ): void {
        if (this.field.isAttached) {
            if (this.wrapper === wrapper && this.cell?.row === cell.row && this.cell.col === cell.col) {
                if (options.clientX != null && options.clientY != null) {
                    this.field.placeCaretAt(options);
                } else {
                    this.field.focusAt();
                }
                return;
            }
            this.releaseWrapper(this.field.detach());
        }

        this.callbacks = callbacks;
        this.cell = cell;
        this.wrapper = wrapper;
        this.field.attach(wrapper, initial, {
            onNavigate: (action) => this.finish(action),
            onBlur: () => this.finish('blur'),
        }, options);

        if (options.clientX == null && options.clientY == null) {
            this.field.placeCaretAtEnd();
        }
    }

    private finish(action: CellNavAction): void {
        if (!this.callbacks || !this.cell) return;
        const value = this.field.detach();
        const callbacks = this.callbacks;
        this.releaseWrapper(value);
        callbacks.onDone(action, value);
    }

    private releaseWrapper(restoreText: string): void {
        if (this.wrapper) {
            setWrapperText(this.wrapper, restoreText);
            this.wrapper = null;
        }
        this.cell = null;
        this.callbacks = null;
    }
}
