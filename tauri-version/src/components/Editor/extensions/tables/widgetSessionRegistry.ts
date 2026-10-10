import type { TableModel } from './model';

export interface TableWidgetHandle {
    from: number;
    contentTo: number;
    blockTo: number;
}

interface WidgetSessionHost {
    dispose(): void;
    syncHandle(handle: TableWidgetHandle): void;
    syncModel(model: TableModel): void;
}

const sessions = new WeakMap<HTMLElement, WidgetSessionHost>();

export function bindWidgetSession(root: HTMLElement, session: WidgetSessionHost): void {
    sessions.set(root, session);
}

export function unbindWidgetSession(dom: HTMLElement): void {
    sessions.get(dom)?.dispose();
    sessions.delete(dom);
}

export function syncWidgetSession(
    dom: HTMLElement,
    handle: TableWidgetHandle,
    model?: TableModel,
): boolean {
    const session = sessions.get(dom);
    if (!session) return false;
    session.syncHandle(handle);
    if (model) session.syncModel(model);
    return true;
}

export function cellRefFromElement(
    root: HTMLElement,
    target: EventTarget | null,
): { row: number; col: number } | null {
    if (!(target instanceof HTMLElement)) return null;
    const cell = target.closest('td') as HTMLTableCellElement | null;
    if (!cell || !root.contains(cell)) return null;
    const row = Number(cell.dataset.row);
    const col = Number(cell.dataset.col);
    if (!Number.isFinite(row) || !Number.isFinite(col)) return null;
    return { row, col };
}
