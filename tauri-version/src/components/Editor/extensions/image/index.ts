import type { Extension } from '@codemirror/state';
import { imageEmbedPaste } from './paste';
import { imageFocusExtension } from './focus';
import { imageEmbedTheme } from './theme';

export const imageEmbedExtension: Extension = [
  imageFocusExtension,
  imageEmbedPaste,
  imageEmbedTheme,
];
