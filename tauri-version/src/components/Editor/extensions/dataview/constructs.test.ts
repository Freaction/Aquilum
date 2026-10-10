import { describe, expect, it } from 'vitest';
import { markdown, markdownLanguage } from '@codemirror/lang-markdown';
import { EditorState } from '@codemirror/state';
import { editorMarkdownExtensions } from '../markdownConfig';
import { dataviewQueriesInText, findDataviewBlocks } from './constructs';

function stateOf(doc: string) {
  return EditorState.create({
    doc,
    extensions: [
      markdown({ base: markdownLanguage, addKeymap: false, extensions: editorMarkdownExtensions }),
    ],
  });
}

function blocks(doc: string) {
  return findDataviewBlocks(stateOf(doc));
}

const NOTE = [
  '# Заметка',
  '',
  '```dataview',
  'TABLE WITHOUT ID',
  'link(file.link, title) AS "Ссылающиеся заметки"',
  'FROM [[Пороки]]',
  'SORT file.ctime DESC',
  '```',
  '',
  'хвост',
].join('\n');

describe('dataviewQueriesInText', () => {
  const cases: Record<string, string> = {
    'обычная заметка': NOTE,
    'два блока': ['```dataview', 'LIST', '```', '', '```dataview', 'TABLE', '```'].join('\n'),
    'другой язык рядом': ['```js', 'const a = 1;', '```', '', '```dataview', 'LIST', '```'].join('\n'),
    'любой регистр': '```DataView\nLIST\n```',
    'пустой блок': '```dataview\n```',
    'незакрытый блок': ['```dataview', 'LIST', '', 'текст после'].join('\n'),
    'тильды': ['~~~dataview', 'LIST', '~~~'].join('\n'),
    'без блоков': '# Заголовок\n\nтекст',
  };

  for (const [name, doc] of Object.entries(cases)) {
    it(`совпадает с поиском по дереву: ${name}`, () => {
      const byTree = findDataviewBlocks(stateOf(doc))
        .map((block) => block.query)
        .filter((query) => query.length > 0);
      expect(dataviewQueriesInText(doc)).toEqual(byTree);
    });
  }
});

describe('findDataviewBlocks', () => {
  it('takes the query without the fence lines', () => {
    const found = blocks(NOTE);
    expect(found).toHaveLength(1);
    expect(found[0].query).toBe([
      'TABLE WITHOUT ID',
      'link(file.link, title) AS "Ссылающиеся заметки"',
      'FROM [[Пороки]]',
      'SORT file.ctime DESC',
    ].join('\n'));
  });

  it('covers the whole block so the widget replaces it entirely', () => {
    const [block] = blocks(NOTE);
    const opening = NOTE.indexOf('```dataview');
    const closing = NOTE.lastIndexOf('```') + 3;
    expect(block.from).toBe(opening);
    expect(block.to).toBe(closing);
  });

  it('ignores fences of other languages', () => {
    expect(blocks('```js\nconst a = 1;\n```')).toHaveLength(0);
    expect(blocks('```\nTABLE\n```')).toHaveLength(0);
  });

  it('reads the language regardless of case', () => {
    expect(blocks('```DataView\nLIST\n```')).toHaveLength(1);
  });

  it('finds every block of the note', () => {
    const doc = ['```dataview', 'LIST', '```', '', '```dataview', 'TABLE', '```'].join('\n');
    const found = blocks(doc);
    expect(found.map((block) => block.query)).toEqual(['LIST', 'TABLE']);
  });

  it('leaves an unfinished block with an empty query instead of eating the note', () => {
    const doc = ['```dataview', 'LIST', '', 'текст после'].join('\n');
    const found = blocks(doc);
    expect(found).toHaveLength(1);
    expect(found[0].query).toContain('LIST');
    expect(found[0].query).not.toContain('```');
  });

  it('has nothing to run for an empty block', () => {
    expect(blocks('```dataview\n```')[0].query).toBe('');
  });
});
