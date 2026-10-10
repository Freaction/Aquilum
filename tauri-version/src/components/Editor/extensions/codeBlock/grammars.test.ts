import { describe, expect, it } from 'vitest';
import { EditorState } from '@codemirror/state';
import { ensureSyntaxTree, HighlightStyle, syntaxTree } from '@codemirror/language';
import { highlightTree, tags } from '@lezer/highlight';
import { codeBlocks, isFenceMark } from './blocks';
import { fenceLanguage } from './languages';
import { isGrammarMounted } from './grammars';
import { editorMarkdownSupport } from '../markdownConfig';

const DOC = ['текст', '', '```js', 'const a = 1;', '```', ''].join('\n');

const keywords = HighlightStyle.define([{ tag: tags.keyword, class: 'q-code-keyword' }]);

function stateOf(doc: string) {
  return EditorState.create({ doc, extensions: [editorMarkdownSupport] });
}

function firstBlock(state: EditorState) {
  return codeBlocks(syntaxTree(state), state.doc, [{ from: 0, to: state.doc.length }])[0]!;
}

function keywordCount(state: EditorState) {
  let count = 0;
  highlightTree(syntaxTree(state), keywords, () => { count += 1; });
  return count;
}

describe('code block grammars', () => {
  it('matches the fence info against a language description', () => {
    expect(fenceLanguage('js')?.name).toBe('JavaScript');
    expect(fenceLanguage('ts')?.name).toBe('TypeScript');
    expect(fenceLanguage('')).toBeNull();
    expect(fenceLanguage('какой-то текст')).toBeNull();
  });

  it('reports the grammar as mounted once the language has loaded and the doc is reparsed', async () => {
    await fenceLanguage('js')!.load();
    const state = stateOf(DOC);
    ensureSyntaxTree(state, DOC.length, 5000);
    const reparsed = state.update({ changes: { from: 0, insert: 'x' } }).state;
    const block = firstBlock(reparsed);
    expect(isGrammarMounted(syntaxTree(reparsed), reparsed.doc, block)).toBe(true);
    expect(keywordCount(reparsed)).toBe(1);
  });

  it('treats a block without code lines as nothing left to mount', () => {
    const state = stateOf('```js\n```\n');
    const block = firstBlock(state);
    expect(isGrammarMounted(syntaxTree(state), state.doc, block)).toBe(true);
  });

  it('recognises fence marks so live preview leaves them visible', () => {
    const state = stateOf(DOC);
    const tree = syntaxTree(state);
    const marks: boolean[] = [];
    tree.iterate({
      enter: (node) => {
        if (node.name === 'CodeMark') marks.push(isFenceMark(node));
      },
    });
    expect(marks).toEqual([true, true]);
  });

  it('does not treat inline code marks as fence marks', () => {
    const state = stateOf('обычный `код` в тексте');
    const tree = syntaxTree(state);
    const marks: boolean[] = [];
    tree.iterate({
      enter: (node) => {
        if (node.name === 'CodeMark') marks.push(isFenceMark(node));
      },
    });
    expect(marks).toEqual([false, false]);
  });
});
