import type { EditorView } from '@codemirror/view';
import { advancedTablesConfig, transformTable } from './advancedTables';
import { focusAfterTable } from './apply';
import { TableCellEditor, type CellFlush, type CellNavAction } from './cellEditor';
import { closeTableContextMenu } from './contextMenu';
import { sanitizeCellValue, serializeTable } from './constructs';
import { collapseSelection } from '../editorFocus';
import { setTableInteractionChrome } from './parentSelection';
import {
    anchorAt,
    cellValue,
    rangeContains,
    setCell,
    setColumnAlign,
    unmergeCell,
    type TableAlign,
    type CellRef,
    type TableModel,
} from './model';
import {
    applyBorderResize,
    beginBorderResize,
    hideResizeLine,
    hitTestBorder,
    paintResizeLine,
    paintResizeLineAtIndex,
    type BorderResizeState,
} from './borderResize';
import {
    scheduleApplyTableModel,
    scheduleDeleteTable,
} from './structure';
import {
    findCellWrapper,
    paintCellWrapper,
    paintTableModel,
} from './widgetDom';
import { syncStripSizes } from './geometry';
import { WidgetDocFlush } from './widgetSessionDoc';
import { openWidgetStructureMenu } from './widgetSessionMenu';
import { handleCellNavigation, type CellNavigationHost } from './widgetSessionNav';
import { cellRefFromElement, type TableWidgetHandle } from './widgetSessionRegistry';
import {
    runAddColumn,
    runAddRow,
    runDeleteColumn,
    runDeleteRow,
    type StructureSession,
} from './widgetSessionStructure';
import {
    beginCellPointer,
    canMergeSelection,
    clearSelectionCells,
    clearTableSelection,
    createPointerState,
    mergeSelection,
    normalizeRect,
    selectedCells,
    selectCellRange,
    selectColumn,
    selectRow,
    resetPointerState,
    syncSelectionPaint,
    updatePointerDrag,
    type TablePointerState,
    type TableSelection,
    type TableSelectionHost,
} from './selection';
import {
    beginStripDrag,
    clearStripDrag,
    commitStripDrag,
    expandedColBlock,
    expandedRowBlock,
    updateStripDrag,
    type StripDragState,
} from './stripReorder';

const DOC_FLUSH_MS = 250;

export class WidgetSession implements CellNavigationHost, StructureSession, TableSelectionHost {
    private readonly editor: TableCellEditor;
    private readonly fieldHost: HTMLElement;
    readonly docFlush: WidgetDocFlush;
    readonly view: EditorView;
    readonly root: HTMLElement;
    private tableModel: TableModel;
    private tableSelection: TableSelection | null = null;
    private readonly pointer: TablePointerState = createPointerState();
    private borderResize: BorderResizeState | null = null;
    private stripDrag: StripDragState | null = null;
    private readonly resizeObserver = new ResizeObserver(() => {
        if (this.disposed) return;
        syncStripSizes(this.root, this.tableModel);
        this.syncSelectionOverlay();
        this.view.requestMeasure();
    });
    private disposed = false;

    constructor(
        view: EditorView,
        private widget: TableWidgetHandle,
        model: TableModel,
        root: HTMLElement,
    ) {
        this.view = view;
        this.root = root;
        this.tableModel = model;
        this.fieldHost = document.createElement('div');
        this.fieldHost.hidden = true;
        this.root.appendChild(this.fieldHost);
        this.editor = new TableCellEditor(
            view,
            this.fieldHost,
            (focused) => this.setNestedFocus(focused),
        );
        this.docFlush = new WidgetDocFlush(() => this.apply(), DOC_FLUSH_MS);
        this.resizeObserver.observe(this.root);
        document.addEventListener('mousemove', this.onDocumentMove);
        document.addEventListener('mouseup', this.onDocumentUp);
        this.root.addEventListener('focusout', this.onTableBlur);
    }

    private readonly onDocumentMove = (event: MouseEvent): void => {
        this.onMouseMove(event);
    };

    private readonly onDocumentUp = (event: MouseEvent): void => {
        this.onMouseUp(event);
    };

    private readonly onDocumentMouseDown = (event: MouseEvent): void => {
        if (this.disposed || !this.tableSelection || event.button !== 0) return;
        if (event.target instanceof Element && event.target.closest('.q-menu')) return;
        if (event.shiftKey && event.target instanceof Node && this.root.contains(event.target)) return;
        clearTableSelection(this);
    };

    get isDisposed(): boolean {
        return this.disposed;
    }

    setModel(model: TableModel): void {
        this.tableModel = model;
    }

    syncHandle(handle: TableWidgetHandle): void {
        this.widget = handle;
    }

    syncModel(model: TableModel): void {
        if (serializeTable(this.tableModel) === serializeTable(model)) {
            this.tableModel = model;
            return;
        }
        this.absorbActiveCell();
        this.tableModel = model;
        clearTableSelection(this);
        paintTableModel(this.root, model);
        this.view.requestMeasure();
    }

    onKeyDown(event: KeyboardEvent): void {
        if (this.disposed) return;
        if (event.key === 'Escape' && this.tableSelection) {
            event.preventDefault();
            event.stopPropagation();
            clearTableSelection(this);
            return;
        }
        if (event.key !== 'Delete' && event.key !== 'Backspace') return;
        if (this.editor.activeCell) return;
        if (selectedCells(this).length === 0) return;
        event.preventDefault();
        event.stopPropagation();
        clearSelectionCells(this);
    }

    onMouseDown(event: MouseEvent): void {
        if (this.disposed) return;
        const target = event.target as HTMLElement | null;
        if (!target) return;

        if (
            target.closest('.q-md-table-scroll')
            && !target.closest('.q-md-table-layout, .q-md-table-add-row, .q-md-table-add-col')
        ) {
            return;
        }

        closeTableContextMenu();
        collapseSelection(this.view);
        resetPointerState(this.pointer);

        if (event.button === 0) {
            const borderHit = hitTestBorder(this.root, event.clientX, event.clientY);
            if (borderHit) {
                event.preventDefault();
                event.stopPropagation();
                this.absorbActiveCell();
                clearTableSelection(this);
                this.borderResize = beginBorderResize(borderHit, this.root, this.tableModel, event);
                paintResizeLine(this.root, borderHit);
                return;
            }
        }

        const colStrip = target.closest('.q-md-table-col-strip-btn') as HTMLElement | null;
        if (colStrip) {
            event.preventDefault();
            event.stopPropagation();
            const col = Number(colStrip.dataset.col);
            if (event.shiftKey) {
                selectColumn(this, col, true);
                return;
            }
            selectColumn(this, col, false);
            hideResizeLine(this.root);
            const block = expandedColBlock(this.tableModel, col);
            this.stripDrag = beginStripDrag('col', block.left, block.right, event);
            return;
        }

        const rowStrip = target.closest('.q-md-table-row-strip-btn') as HTMLElement | null;
        if (rowStrip) {
            event.preventDefault();
            event.stopPropagation();
            const row = Number(rowStrip.dataset.row);
            if (event.shiftKey) {
                selectRow(this, row, true);
                return;
            }
            selectRow(this, row, false);
            hideResizeLine(this.root);
            const block = expandedRowBlock(this.tableModel, row);
            this.stripDrag = beginStripDrag('row', block.top, block.bottom, event);
            return;
        }

        if (target.closest('.q-md-table-add-row')) {
            event.preventDefault();
            event.stopPropagation();
            this.addRow();
            return;
        }
        if (target.closest('.q-md-table-add-col')) {
            event.preventDefault();
            event.stopPropagation();
            this.addColumn();
            return;
        }

        const cell = cellRefFromElement(this.root, target);
        if (cell) {
            if (event.button === 2) {
                const selection = this.tableSelection;
                const range = selection && normalizeRect(selection.anchor, selection.focus);
                if (range && rangeContains(range, cell.row, cell.col)) {
                    event.preventDefault();
                    event.stopPropagation();
                    return;
                }
                event.preventDefault();
                event.stopPropagation();
                selectCellRange(this, cell, false);
                return;
            }

            if (event.button !== 0) return;

            if (event.shiftKey) {
                event.preventDefault();
                event.stopPropagation();
                selectCellRange(this, cell, true);
                return;
            }

            const active = this.editor.activeCell;
            const sameCell = !!active
                && active.row === cell.row
                && active.col === cell.col;

            if (sameCell && target.closest('.q-md-table-cell-field, .q-md-table-cell-wrapper .cm-content')) {
                beginCellPointer(this.pointer, cell, event);
                return;
            }

            event.preventDefault();
            event.stopPropagation();
            beginCellPointer(this.pointer, cell, event);
            clearTableSelection(this);
            this.activateCell(cell, { clientX: event.clientX, clientY: event.clientY });
            return;
        }

        event.preventDefault();
        clearTableSelection(this);
        this.leaveTable();
    }

    onMouseMove(event: MouseEvent): void {
        if (this.disposed) return;
        if (this.stripDrag) {
            updateStripDrag(this.stripDrag, this.tableModel, this.root, event);
            if (this.stripDrag.dragging) {
                this.root.classList.add('q-md-table-strip-dragging');
                this.root.classList.toggle('q-md-table-strip-drop-invalid', !this.stripDrag.gapValid);
                event.preventDefault();
            }
            return;
        }
        if (this.borderResize) {
            const next = applyBorderResize(this.tableModel, this.borderResize, event);
            if (next) {
                this.tableModel = next;
                paintTableModel(this.root, next);
                syncSelectionPaint(this);
                this.view.requestMeasure();
                this.markDocDirty();
            }
            paintResizeLineAtIndex(this.root, this.borderResize.axis, this.borderResize.index);
            event.preventDefault();
            return;
        }
        if (event.buttons === 0) {
            const hit = hitTestBorder(this.root, event.clientX, event.clientY);
            if (hit) paintResizeLine(this.root, hit);
            else hideResizeLine(this.root);
            return;
        }
        if (event.buttons !== 1) return;
        const focus = cellRefFromElement(this.root, document.elementFromPoint(event.clientX, event.clientY));
        const anchor = this.pointer.anchor;
        const active = this.editor.activeCell;
        if (
            anchor
            && active
            && active.row === anchor.row
            && active.col === anchor.col
            && focus
            && focus.row === anchor.row
            && focus.col === anchor.col
        ) {
            if (Math.hypot(event.clientX - this.pointer.startX, event.clientY - this.pointer.startY) >= 4) {
                this.editor.activeField.selectFromCoords(
                    { clientX: this.pointer.startX, clientY: this.pointer.startY },
                    { clientX: event.clientX, clientY: event.clientY },
                );
                event.preventDefault();
            }
            return;
        }
        if (updatePointerDrag(this, this.pointer, focus, event)) {
            event.preventDefault();
        }
    }

    onMouseUp(event: MouseEvent): void {
        if (this.disposed) return;

        if (this.borderResize) {
            this.borderResize = null;
            hideResizeLine(this.root);
            this.markDocDirty();
            event.preventDefault();
            event.stopPropagation();
            return;
        }

        if (this.stripDrag) {
            const drag = this.stripDrag;
            this.stripDrag = null;
            this.root.classList.remove('q-md-table-strip-dragging', 'q-md-table-strip-drop-invalid');
            clearStripDrag(this.root);
            if (drag.dragging) {
                const result = commitStripDrag(this.tableModel, drag);
                if (result) {
                    this.absorbActiveCell();
                    clearTableSelection(this);
                    this.setModel(result.model);
                    paintTableModel(this.root, result.model);
                    this.view.requestMeasure();
                    this.docFlush.cancel();
                    this.apply();
                    if (drag.axis === 'row') selectRow(this, result.resume.row, false);
                    else selectColumn(this, result.resume.col, false);
                }
                event.preventDefault();
                event.stopPropagation();
            }
            return;
        }

        if (this.pointer.dragging) {
            resetPointerState(this.pointer);
            return;
        }

        const pending = this.pointer.anchor && !this.pointer.dragging ? this.pointer.anchor : null;
        resetPointerState(this.pointer);
        if (!pending) return;

        const active = this.editor.activeCell;
        if (!active || active.row !== pending.row || active.col !== pending.col) return;

        const field = this.editor.activeField;
        if (field?.hasTextSelection()) return;

        event.preventDefault();
        event.stopPropagation();
        field?.placeCaretAt({ clientX: event.clientX, clientY: event.clientY });
    }

    activateCell(cell: CellRef, click?: { clientX: number; clientY: number }): void {
        if (this.disposed) return;

        const active = this.editor.activeCell;
        if (active && (active.row !== cell.row || active.col !== cell.col)) {
            const flushed = this.editor.flush();
            if (flushed) this.commitFlushed(flushed);
        }

        this.openCellEditor(cell, click);
    }

    dispose(): void {
        if (this.disposed) return;
        this.disposed = true;
        this.resizeObserver.disconnect();
        document.removeEventListener('mousemove', this.onDocumentMove);
        document.removeEventListener('mouseup', this.onDocumentUp);
        this.root.removeEventListener('focusout', this.onTableBlur);
        this.syncSelectionClearListener(false);
        this.docFlush.cancel();
        closeTableContextMenu();
        clearStripDrag(this.root);
        this.stripDrag = null;
        this.borderResize = null;
        hideResizeLine(this.root);
        this.root.classList.remove('q-md-table-strip-dragging', 'q-md-table-strip-drop-invalid');
        clearTableSelection(this);
        this.setNestedFocus(false);
        setTableInteractionChrome(this.view, false);

        const flushed = this.editor.flush();
        if (flushed) this.commitFlushed(flushed);

        if (this.docFlush.isDirty) {
            scheduleApplyTableModel(
                this.view,
                this.widget.from,
                this.widget.contentTo,
                this.tableModel,
            );
        }

        this.editor.destroy();
        this.fieldHost.remove();
    }

    onContextMenu(event: MouseEvent): void {
        if (this.disposed) return;
        event.preventDefault();
        event.stopPropagation();
        collapseSelection(this.view);

        const cell = cellRefFromElement(this.root, event.target);
        if (!cell) return;

        openWidgetStructureMenu(event.clientX, event.clientY, cell, this.tableModel, {
            canMerge: canMergeSelection(this),
            ...(this.view.state.facet(advancedTablesConfig)?.enabled ? { advanced: {
                format: () => this.transform(),
                sort: (descending: boolean) => this.transform(cell.col, descending),
                readOnly: this.view.state.readOnly,
            } } : {}),
            actions: {
                onMerge: () => {
                    if (mergeSelection(this)) clearTableSelection(this);
                },
                onAlignColumn: (align) => this.alignColumn(cell.col, align),
                onUnmerge: () => this.unmerge(cell),
                onDeleteRow: () => this.deleteRowAt(cell.row, cell.col),
                onDeleteColumn: () => this.deleteColumnAt(cell.row, cell.col),
                onDeleteTable: () => this.deleteTable(),
            },
        });
    }

    leaveTable(): void {
        const flushed = this.editor.flush();
        if (flushed) this.commitFlushed(flushed);
        this.docFlush.flushNow();
        focusAfterTable(this.view, this.widget.blockTo);
        this.onTableBlur();
    }

    commitCell(cell: CellRef, value: string): { prev: string; next: string } {
        const prev = cellValue(this.tableModel, cell.row, cell.col);
        const next = sanitizeCellValue(value);
        this.tableModel = setCell(this.tableModel, cell.row, cell.col, next);
        paintCellWrapper(this.root, cell, next);
        return { prev, next };
    }

    applyStructure(resume: CellRef): void {
        const target = anchorAt(this.tableModel, resume.row, resume.col) ?? resume;
        clearTableSelection(this);
        paintTableModel(this.root, this.tableModel);
        this.view.requestMeasure();
        this.docFlush.cancel();
        this.apply(target);
        queueMicrotask(() => {
            if (this.disposed) return;
            this.openCellEditor(target);
        });
    }

    openCellEditor(cell: CellRef, click?: { clientX: number; clientY: number }): void {
        const target = anchorAt(this.tableModel, cell.row, cell.col);
        if (!target) return;
        const wrapper = findCellWrapper(this.root, target);
        if (!wrapper) return;
        const initial = cellValue(this.tableModel, target.row, target.col);
        this.editor.activate(wrapper, target, initial, {
            onDone: (action, value) => this.onCellDone(target, action, value),
        }, click);
    }

    private setNestedFocus(focused: boolean): void {
        this.view.dom.classList.toggle('q-md-table-cell-focused', focused);
        this.syncInteractionChrome();
    }

    private syncInteractionChrome(): void {
        const selecting = selectedCells(this).length > 0;
        const editing = this.editor.activeCell !== null;
        setTableInteractionChrome(this.view, selecting || editing);
    }

    private syncSelectionOverlay(): void {
        if (this.disposed || !this.tableSelection) return;
        syncSelectionPaint(this);
    }

    getSelection(): TableSelection | null {
        return this.tableSelection;
    }

    setSelection(selection: TableSelection | null): void {
        this.tableSelection = selection;
        syncSelectionPaint(this);
        this.syncSelectionClearListener(selection !== null);
        if (selection) {
            this.root.focus({ preventScroll: true });
        } else {
            this.syncInteractionChrome();
        }
    }

    markDocDirty(): void {
        this.docFlush.markDirty();
    }

    repaintModel(): void {
        paintTableModel(this.root, this.tableModel);
        syncSelectionPaint(this);
        this.view.requestMeasure();
    }

    private apply(resume?: CellRef): void {
        scheduleApplyTableModel(
            this.view,
            this.widget.from,
            this.widget.contentTo,
            this.tableModel,
            resume,
        );
        this.docFlush.clearDirty();
    }

    private transform(column?: number, descending = false): void {
        if (this.disposed || this.view.state.readOnly) return;
        this.absorbActiveCell();
        this.docFlush.flushNow();
        queueMicrotask(() => {
            if (!this.disposed) void transformTable(this.view, this.widget.from, this.root, column, descending);
        });
    }

    private formatQueued = false;
    private onTableBlur = (): void => {
        if (this.formatQueued) return;
        this.formatQueued = true;
        queueMicrotask(() => {
            this.formatQueued = false;
            if (this.disposed || !this.view.state.facet(advancedTablesConfig)?.formatOnLeave) return;
            const focused = document.activeElement;
            if (this.root.contains(focused) || focused?.closest('[role="menu"]')) return;
            this.transform();
        });
    };

    private onCellDone(cell: CellRef, action: CellNavAction, value: string): void {
        const advanced = this.view.state.facet(advancedTablesConfig);
        if (action === 'escape' && advanced?.enabled && advanced.formatOnLeave) this.docFlush.markDirty();
        handleCellNavigation(this, cell, action, value);
        if (action === 'blur') this.onTableBlur();
    }

    private commitFlushed(flushed: CellFlush): void {
        const { prev, next } = this.commitCell(flushed.cell, flushed.value);
        if (next !== prev) this.docFlush.markDirty();
    }

    getModel(): TableModel {
        return this.tableModel;
    }

    absorbActiveCell(): void {
        const flushed = this.editor.flush();
        if (!flushed) return;
        this.commitFlushed(flushed);
    }

    private syncSelectionClearListener(active: boolean): void {
        if (active) document.addEventListener('mousedown', this.onDocumentMouseDown, true);
        else document.removeEventListener('mousedown', this.onDocumentMouseDown, true);
    }

    private addRow(): void {
        runAddRow(this);
    }

    private addColumn(): void {
        runAddColumn(this);
    }

    private deleteRowAt(row: number, col: number): void {
        runDeleteRow(this, row, col);
    }

    private deleteColumnAt(row: number, col: number): void {
        runDeleteColumn(this, row, col);
    }

    private deleteTable(): void {
        this.docFlush.cancel();
        this.docFlush.clearDirty();
        this.editor.flush();
        this.setNestedFocus(false);
        scheduleDeleteTable(this.view, this.widget.from, this.widget.blockTo);
    }

    private alignColumn(col: number, align: TableAlign): void {
        this.absorbActiveCell();
        const next = setColumnAlign(this.tableModel, col, align);
        if (!next) return;
        this.tableModel = next;
        this.applyStructure({ row: 0, col });
    }

    private unmerge(cell: CellRef): void {
        this.absorbActiveCell();
        const resume = anchorAt(this.tableModel, cell.row, cell.col) ?? cell;
        const next = unmergeCell(this.tableModel, cell.row, cell.col);
        if (!next) return;
        this.tableModel = next;
        this.applyStructure(resume);
    }
}
