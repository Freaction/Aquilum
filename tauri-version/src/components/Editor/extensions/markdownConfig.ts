import { markdown, markdownLanguage } from '@codemirror/lang-markdown';
import { languages } from '@codemirror/language-data';
import type { MarkdownConfig } from '@lezer/markdown';
import { plainBracketMarkdownConfig, wikiLinkMarkdownConfig } from './links';
import { outlineMarkdownConfig } from './outline';

export const editorMarkdownExtensions: readonly MarkdownConfig[] = [
  outlineMarkdownConfig,
  wikiLinkMarkdownConfig,
  plainBracketMarkdownConfig,
];

export const editorMarkdownSupport = markdown({
  base: markdownLanguage,
  addKeymap: false,
  codeLanguages: languages,
  extensions: editorMarkdownExtensions,
});
