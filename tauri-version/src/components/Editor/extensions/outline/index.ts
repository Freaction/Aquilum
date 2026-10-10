import { outlineKeymap } from './commands';
import { outlinePreview } from './preview';
import { outlineTheme } from './theme';
import { outlineRenumbering } from './renumber';

export const outlineExtension = [
    outlineKeymap,
    outlinePreview,
    outlineRenumbering,
    outlineTheme,
];

export { listCalloutsExtension } from './listCallouts';
export { outlineMarkdownConfig } from './constructs';
