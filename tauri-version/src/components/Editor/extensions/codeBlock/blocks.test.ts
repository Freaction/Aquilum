import { describe, expect, it } from 'vitest';
import { markdown, markdownLanguage } from '@codemirror/lang-markdown';
import { EditorState } from '@codemirror/state';
import { syntaxTree } from '@codemirror/language';
import { codeBlocks } from './blocks';
import { editorMarkdownExtensions } from '../markdownConfig';

function blocks(doc: string) {
  const state = EditorState.create({
    doc,
    extensions: [markdown({ base: markdownLanguage, addKeymap: false, extensions: editorMarkdownExtensions })],
  });
  return codeBlocks(syntaxTree(state), state.doc, [{ from: 0, to: doc.length }]);
}

const FENCED = [
  'текст',
  '',
  '```js',
  'const a = 1;',
  'const b = 2;',
  '```',
  '',
].join('\n');

describe('code blocks', () => {
  it('spans a fenced block from its opening line to the end of its closing line', () => {
    expect(blocks(FENCED)).toEqual([{ from: 7, end: 42, info: 'js' }]);
  });

  it('leaves prose and inline code alone', () => {
    expect(blocks('обычный `код` в тексте')).toEqual([]);
  });

  it('stops at the last line of a block whose fence is not closed yet', () => {
    expect(blocks('```\n')).toEqual([{ from: 0, end: 3, info: '' }]);
  });

  it('covers an indented code block too', () => {
    expect(blocks('\tcode\n')).toEqual([{ from: 0, end: 5, info: '' }]);
  });

  it('keeps two adjacent blocks apart', () => {
    const doc = ['```js', 'a', '```', '```py', 'b', '```', ''].join('\n');
    expect(blocks(doc)).toEqual([{ from: 0, end: 11, info: 'js' }, { from: 12, end: 23, info: 'py' }]);
  });
});
