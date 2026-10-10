import { describe, expect, it } from 'vitest';
import { LanguageDescription } from '@codemirror/language';
import { languages } from '@codemirror/language-data';
import { editorMarkdownSupport } from './markdownConfig';

describe('nested code languages', () => {
  it('parses a fenced block with the language named in its info string', async () => {
    const javascript = LanguageDescription.matchLanguageName(languages, 'js', true);
    await javascript?.load();

    const doc = ['```js', 'const a = 1;', '```', ''].join('\n');
    const tree = editorMarkdownSupport.language.parser.parse(doc);

    expect(tree.resolveInner(8, 1).name).toBe('const');
  });

  it('leaves a block without an info string to the markdown parser', () => {
    const doc = ['```', 'const a = 1;', '```', ''].join('\n');
    const tree = editorMarkdownSupport.language.parser.parse(doc);

    expect(tree.resolveInner(5, 1).name).toBe('CodeText');
  });
});
