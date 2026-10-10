import { t } from '../../../../i18n';
import { closeImperativeMenu, showImperativeMenu } from '../../../Common/imperativeMenu';
import type { MenuItem } from '../../../Common/Menu';

export function closeTableContextMenu(): void {
    closeImperativeMenu();
}

export function showTableContextMenu(x: number, y: number, items: MenuItem[]): void {
    showImperativeMenu(x, y, items, t('editor.table.menuAria'));
}
