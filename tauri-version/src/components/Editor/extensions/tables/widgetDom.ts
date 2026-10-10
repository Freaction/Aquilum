import { tableCellParser } from './rows';
import { linkLabelRange } from '../links/flow';
import { t } from '../../../../i18n';
import { Plus } from 'lucide';
import { createIconElement } from '../../../Common/iconElement';
import {
    anchorAt,
    columnCount,
    mergeAt,
    rangesIntersect,
    rowCount,
    totalColWidth,
    type CellRef,
    type MergeRange,
    type TableModel,
} from './model';
import { paintStripLabels } from './widgetDomStrips';
import { syncStripSizes } from './geometry';

function createTableAddIcon(): HTMLElement {
    const icon = document.createElement('span');
    icon.className = 'q-md-table-add-icon';
    icon.setAttribute('aria-hidden', 'true');
    icon.appendChild(createIconElement(Plus, 1.5));
    return icon;
}

export function setWrapperText(wrapper: HTMLElement, text: string): void {
    wrapper.replaceChildren();
    if (!text) {
        wrapper.appendChild(document.createElement('br'));
        return;
    }

    const display = text.replace(/\\\|/g, '|');
    let offset = 0;
    tableCellParser.parse(display).iterate({
        enter(node) {
            if (node.name === 'InlineCode') {
                const marks = node.node.getChildren('CodeMark');
                if (marks.length < 2) return false;
                wrapper.append(document.createTextNode(display.slice(offset, node.from)));
                const code = document.createElement('span');
                code.className = 'q-md-code';
                code.textContent = display.slice(marks[0].to, marks[marks.length - 1].from);
                wrapper.append(code);
                offset = node.to;
                return false;
            }
            if (['FencedCode', 'CodeBlock', 'Image'].includes(node.name)) return false;
            const wiki = node.name === 'WikiLink';
            if (!wiki && node.name !== 'Link') return;
            const target = node.node.getChild(wiki ? 'WikiLinkTarget' : 'URL');
            const label = wiki ? node.node.getChild('WikiLinkAlias') ?? target : linkLabelRange(node.node);
            if (!target || !label) return;
            wrapper.append(document.createTextNode(display.slice(offset, node.from)));
            const link = document.createElement('a');
            link.className = 'q-md-link q-md-table-link';
            const destination = display.slice(target.from, target.to).trim();
            if (wiki) link.dataset.wikiTarget = destination;
            else link.dataset.externalUrl = destination;
            link.href = wiki ? '#' : destination;
            link.textContent = display.slice(label.from, label.to).trim() || destination;
            wrapper.append(link);
            offset = node.to;
            return false;
        },
    });
    wrapper.append(document.createTextNode(display.slice(offset)));
}

function buildRow(model: TableModel, row: number): HTMLTableRowElement {
    const tr = document.createElement('tr');
    tr.style.height = `${model.rowHeights[row]}px`;
    const cols = columnCount(model);
    for (let col = 0; col < cols; col += 1) {
        const anchor = anchorAt(model, row, col);
        if (!anchor || anchor.row !== row || anchor.col !== col) continue;
        const merge = mergeAt(model, row, col);

        const el = document.createElement('td');
        el.className = 'q-md-table-cell';
        el.dataset.row = String(row);
        el.dataset.col = String(col);
        if (merge) {
            el.rowSpan = merge.bottom - merge.top + 1;
            el.colSpan = merge.right - merge.left + 1;
        }

        const align = model.aligns[col];
        if (align) el.style.textAlign = align;

        const wrapper = document.createElement('div');
        wrapper.className = 'q-md-table-cell-wrapper';
        setWrapperText(wrapper, model.cells[row][col]);
        el.appendChild(wrapper);
        tr.appendChild(el);
    }
    return tr;
}

function syncColumns(table: HTMLTableElement, model: TableModel): void {
    const explicit = model.colSizing === 'explicit';
    const colgroup = document.createElement('colgroup');
    for (let col = 0; col < columnCount(model); col += 1) {
        const el = document.createElement('col');
        if (explicit) el.style.width = `${model.colWidths[col]}px`;
        colgroup.appendChild(el);
    }
    const current = table.querySelector('colgroup');
    if (current) current.replaceWith(colgroup);
    else table.insertBefore(colgroup, table.firstChild);
    table.style.width = explicit ? `${totalColWidth(model)}px` : '100%';
}

function syncTrackWidths(root: HTMLElement, model: TableModel): void {
    const explicit = model.colSizing === 'explicit';
    const content = explicit ? totalColWidth(model) : 0;
    const withChrome =
        `calc(${content}px + var(--q-md-table-corner-width, var(--q-space-24)) + var(--q-gap-xs))`;
    root.style.setProperty('--q-md-table-layout-min-width', withChrome);
}

function placeSelectionOverlay(root: HTMLElement): void {
    const overlay = root.querySelector('.q-md-table-selection-overlay');
    const host = root.querySelector('.q-md-table-table-host');
    if (overlay && host && overlay.parentElement !== host) host.appendChild(overlay);
}

export function paintTableModel(root: HTMLElement, model: TableModel): void {
    const table = root.querySelector('.q-md-table-editor') as HTMLTableElement | null;
    if (!table) return;

    syncTrackWidths(root, model);
    syncColumns(table, model);
    let tbody = table.querySelector('tbody');
    if (!tbody) {
        tbody = document.createElement('tbody');
        table.appendChild(tbody);
    }

    const rows: HTMLTableRowElement[] = [];
    for (let r = 0; r < rowCount(model); r += 1) {
        rows.push(buildRow(model, r));
    }
    tbody.replaceChildren(...rows);
    paintStripLabels(root, model);
    placeSelectionOverlay(root);
    syncStripSizes(root, model);
}

export function renderTableWidgetDom(model: TableModel): HTMLElement {
    const root = document.createElement('div');
    root.className = 'q-md-table-widget';
    root.contentEditable = 'false';

    const shell = document.createElement('div');
    shell.className = 'q-md-table-shell';

    const scroll = document.createElement('div');
    scroll.className = 'q-md-table-scroll';

    const table = document.createElement('table');
    table.className = 'q-md-table-editor';
    table.tabIndex = -1;
    table.appendChild(document.createElement('tbody'));

    const selection = document.createElement('div');
    selection.className = 'q-md-table-selection-overlay';
    selection.hidden = true;

    const addCol = document.createElement('div');
    addCol.className = 'q-md-table-add-col';
    addCol.setAttribute('aria-label', t('editor.table.addColumn'));
    addCol.appendChild(createTableAddIcon());

    const addRow = document.createElement('div');
    addRow.className = 'q-md-table-add-row';
    addRow.setAttribute('aria-label', t('editor.table.addRow'));
    addRow.appendChild(createTableAddIcon());

    scroll.append(table, selection, addRow);
    shell.append(scroll, addCol);
    root.appendChild(shell);

    paintTableModel(root, model);
    return root;
}

export function findCellWrapper(root: HTMLElement, cell: CellRef): HTMLElement | null {
    const selector = `.q-md-table-cell[data-row="${cell.row}"][data-col="${cell.col}"] .q-md-table-cell-wrapper`;
    return root.querySelector(selector) as HTMLElement | null;
}

export function paintCellWrapper(root: HTMLElement, cell: CellRef, text: string): void {
    const wrapper = findCellWrapper(root, cell);
    if (wrapper) setWrapperText(wrapper, text);
}

export function paintTableSelection(root: HTMLElement, range: MergeRange | null): void {
    const overlay = root.querySelector('.q-md-table-selection-overlay') as HTMLElement | null;
    if (!overlay) return;
    if (!range) {
        overlay.hidden = true;
        return;
    }

    const selected = [...root.querySelectorAll<HTMLTableCellElement>('.q-md-table-cell')]
        .filter((cell) => {
            const row = Number(cell.dataset.row);
            const col = Number(cell.dataset.col);
            const span = { top: row, left: col, bottom: row + cell.rowSpan - 1, right: col + cell.colSpan - 1 };
            return rangesIntersect(span, range);
        });
    if (!selected.length || !overlay.parentElement) {
        overlay.hidden = true;
        return;
    }

    const parentRect = overlay.parentElement.getBoundingClientRect();
    const rects = selected.map((cell) => cell.getBoundingClientRect());
    const left = Math.min(...rects.map((rect) => rect.left)) - parentRect.left;
    const top = Math.min(...rects.map((rect) => rect.top)) - parentRect.top;
    const right = Math.max(...rects.map((rect) => rect.right)) - parentRect.left;
    const bottom = Math.max(...rects.map((rect) => rect.bottom)) - parentRect.top;
    overlay.style.left = `${left}px`;
    overlay.style.top = `${top}px`;
    overlay.style.width = `${right - left}px`;
    overlay.style.height = `${bottom - top}px`;
    overlay.hidden = false;
}
