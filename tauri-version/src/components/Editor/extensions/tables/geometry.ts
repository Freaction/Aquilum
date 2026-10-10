import { columnCount, type TableModel } from './model';

function measuredSize(size: number): number | null {
    return Number.isFinite(size) && size > 0 ? size : null;
}

export function tableOverlayHost(root: HTMLElement): HTMLElement {
    return (root.querySelector('.q-md-table-layout') as HTMLElement | null) ?? root;
}

export function measureColWidths(root: HTMLElement, model: TableModel): number[] {
    const measured = new Array<number | null>(columnCount(model)).fill(null);
    const cells = root.querySelectorAll<HTMLTableCellElement>('.q-md-table-editor tbody .q-md-table-cell');
    for (const cell of cells) {
        if (cell.colSpan !== 1) continue;
        const col = Number(cell.dataset.col);
        if (!Number.isInteger(col) || col < 0 || col >= measured.length) continue;
        if (measured[col] != null) continue;
        measured[col] = measuredSize(cell.getBoundingClientRect().width);
    }
    return model.colWidths.map((requested, col) => measured[col] ?? requested);
}

export function measureRowHeights(root: HTMLElement, model: TableModel): number[] {
    const rows = [...root.querySelectorAll<HTMLTableRowElement>('.q-md-table-editor tbody > tr')];
    return model.rowHeights.map(
        (requested, row) => {
            const rect = rows[row]?.getBoundingClientRect();
            return (rect && measuredSize(rect.height)) ?? requested;
        },
    );
}

function syncCornerWidth(root: HTMLElement): void {
    const strip = root.querySelector('.q-md-table-row-strip');
    const corner = root.querySelector('.q-md-table-corner') as HTMLElement | null;
    if (!strip || !corner) return;
    const width = measuredSize(strip.getBoundingClientRect().width);
    if (width == null) return;
    corner.style.width = `${width}px`;
    root.style.setProperty('--q-md-table-corner-width', `${width}px`);
}

function syncAddColHeight(root: HTMLElement): void {
    const table = root.querySelector('.q-md-table-editor') as HTMLElement | null;
    const scroll = root.querySelector('.q-md-table-scroll');
    const addCol = root.querySelector('.q-md-table-add-col') as HTMLElement | null;
    const addRow = root.querySelector('.q-md-table-add-row') as HTMLElement | null;
    if (!table || !scroll || !addCol || !addRow) return;
    const rect = table.getBoundingClientRect();
    const width = measuredSize(rect.width);
    addRow.style.width = width == null ? table.style.width : `${width}px`;
    const height = measuredSize(rect.height);
    if (height == null) return;
    addCol.style.height = `${height}px`;
    addCol.style.marginTop = `${Math.max(0, rect.top - scroll.getBoundingClientRect().top)}px`;
}

export function syncStripSizes(root: HTMLElement, model: TableModel): void {
    const widths = measureColWidths(root, model);
    const heights = measureRowHeights(root, model);

    for (const button of root.querySelectorAll<HTMLElement>('.q-md-table-col-strip-btn')) {
        const width = widths[Number(button.dataset.col)];
        if (width != null) button.style.width = `${width}px`;
    }
    for (const button of root.querySelectorAll<HTMLElement>('.q-md-table-row-strip-btn')) {
        const height = heights[Number(button.dataset.row)];
        if (height != null) button.style.height = `${height}px`;
    }

    syncCornerWidth(root);
    syncAddColHeight(root);
}
