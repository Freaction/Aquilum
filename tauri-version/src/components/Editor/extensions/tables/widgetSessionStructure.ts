import type { CellRef } from './model';
import type { TableModel } from './model';
import {
    addTableColumn,
    addTableRow,
    removeTableColumn,
    removeTableRow,
} from './structure';

export interface StructureSession {
    absorbActiveCell(): void;
    getModel(): TableModel;
    setModel(model: TableModel): void;
    applyStructure(resume: CellRef): void;
}

export function runAddRow(session: StructureSession): void {
    session.absorbActiveCell();
    const result = addTableRow(session.getModel());
    session.setModel(result.model);
    session.applyStructure(result.resume);
}

export function runAddColumn(session: StructureSession): void {
    session.absorbActiveCell();
    const result = addTableColumn(session.getModel());
    session.setModel(result.model);
    session.applyStructure(result.resume);
}

export function runDeleteRow(session: StructureSession, row: number, col: number): void {
    session.absorbActiveCell();
    const result = removeTableRow(session.getModel(), row, col);
    if (!result) return;
    session.setModel(result.model);
    session.applyStructure(result.resume);
}

export function runDeleteColumn(session: StructureSession, row: number, col: number): void {
    session.absorbActiveCell();
    const result = removeTableColumn(session.getModel(), row, col);
    if (!result) return;
    session.setModel(result.model);
    session.applyStructure(result.resume);
}
