import type { Extension } from '@codemirror/state';
import { tableBoundaryExtension } from './boundary';
import { tablePreview } from './preview';
import { tableTheme } from './theme';

export const tablesExtension: Extension = [
    tablePreview,
    tableTheme,
    tableBoundaryExtension,
];

export { insertTable } from './apply';
