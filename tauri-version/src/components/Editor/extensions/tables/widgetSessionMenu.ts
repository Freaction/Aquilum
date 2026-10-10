import {
    AlignCenter,
    AlignLeft,
    AlignRight,
    Columns3,
    Rows3,
    TableCellsMerge,
    TableCellsSplit,
    Trash2,
    ListOrdered,
    ArrowUp,
    ArrowDown,
} from 'lucide';
import { t } from '../../../../i18n';
import { cellIsMerged, type CellRef } from './model';
import { showTableContextMenu } from './contextMenu';
import { columnAlign, columnCount, rowCount, type TableAlign, type TableModel } from './model';

interface WidgetStructureMenuActions {
    onAlignColumn: (align: TableAlign) => void;
    onMerge: () => void;
    onUnmerge: () => void;
    onDeleteRow: () => void;
    onDeleteColumn: () => void;
    onDeleteTable: () => void;
}

export function openWidgetStructureMenu(
    x: number,
    y: number,
    cell: CellRef,
    model: TableModel,
    options: {
        canMerge: boolean;
        actions: WidgetStructureMenuActions;
        advanced?: { format: () => void; sort: (descending: boolean) => void; readOnly: boolean };
    },
): void {
    const col = cell.col;
    const currentAlign = columnAlign(model, col);
    const merged = cellIsMerged(model, cell.row, col);

    const advanced = options.advanced;
    const sortDisabled = advanced?.readOnly || model.merges.some(merge => merge.top !== merge.bottom);
    showTableContextMenu(x, y, [
        {
            id: 'merge',
            label: t('editor.table.merge'),
            icon: TableCellsMerge,
            disabled: !options.canMerge,
            onSelect: options.actions.onMerge,
        },
        {
            id: 'unmerge',
            label: t('editor.table.unmerge'),
            icon: TableCellsSplit,
            disabled: !merged,
            onSelect: options.actions.onUnmerge,
        },
        {
            id: 'align-left',
            label: t('editor.table.alignLeft'),
            icon: AlignLeft,
            disabled: currentAlign === 'left',
            onSelect: () => options.actions.onAlignColumn('left'),
        },
        {
            id: 'align-center',
            label: t('editor.table.alignCenter'),
            icon: AlignCenter,
            disabled: currentAlign === 'center',
            onSelect: () => options.actions.onAlignColumn('center'),
        },
        {
            id: 'align-right',
            label: t('editor.table.alignRight'),
            icon: AlignRight,
            disabled: currentAlign === 'right',
            onSelect: () => options.actions.onAlignColumn('right'),
        },
        {
            id: 'delete-row',
            label: t('editor.table.deleteRow'),
            icon: Rows3,
            disabled: rowCount(model) <= 2,
            onSelect: options.actions.onDeleteRow,
        },
        {
            id: 'delete-col',
            label: t('editor.table.deleteColumn'),
            icon: Columns3,
            disabled: columnCount(model) <= 1,
            onSelect: options.actions.onDeleteColumn,
        },
        {
            id: 'delete-table',
            label: t('editor.table.deleteTable'),
            icon: Trash2,
            onSelect: options.actions.onDeleteTable,
        },
        ...(advanced ? [
            { id: 'format', label: t('plugins.advancedTables.format'), icon: ListOrdered, disabled: advanced.readOnly, onSelect: advanced.format },
            { id: 'sort-ascending', label: t('plugins.advancedTables.sortAscending'), icon: ArrowUp, disabled: sortDisabled, onSelect: () => advanced.sort(false) },
            { id: 'sort-descending', label: t('plugins.advancedTables.sortDescending'), icon: ArrowDown, disabled: sortDisabled, onSelect: () => advanced.sort(true) },
        ] : []),
    ]);
}
