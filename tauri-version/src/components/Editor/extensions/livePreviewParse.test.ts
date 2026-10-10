import { describe, expect, it } from 'vitest';
import { markdown, markdownLanguage } from '@codemirror/lang-markdown';
import { EditorState } from '@codemirror/state';
import { syntaxTree } from '@codemirror/language';
import { outlineMarkdownConfig } from './outline';
import { wikiLinkMarkdownConfig } from './links';

function dump(doc: string) {
  const state = EditorState.create({
    doc,
    extensions: [markdown({
      base: markdownLanguage,
      extensions: [outlineMarkdownConfig, wikiLinkMarkdownConfig],
    })],
  });
  const rows: string[] = [];
  syntaxTree(state).iterate({
    enter(n) {
      if (
        n.name.includes('List')
        || n.name.includes('Wiki')
        || n.name.includes('Mark')
        || n.name === 'HorizontalRule'
        || n.name === 'Paragraph'
        || n.name.includes('Emphasis')
      ) {
        rows.push(`${n.name}:${n.from}-${n.to}:${JSON.stringify(doc.slice(n.from, n.to)).slice(0, 50)}`);
      }
    },
  });
  return rows.join('\n');
}

describe('parse variants for *** after nested list', () => {
  it('with blank line', () => {
    const doc = '- [[wiki]]\n\n1. a\n\t1. \n\n***';
    // eslint-disable-next-line no-console
    console.log('WITH BLANK\n' + dump(doc));
    expect(dump(doc)).toContain('HorizontalRule');
  });

  it('without blank line', () => {
    const doc = '- [[wiki]]\n\n1. a\n\t1. \n***';
    // eslint-disable-next-line no-console
    console.log('NO BLANK\n' + dump(doc));
    expect(dump(doc)).toBeTruthy();
  });

  it('*** immediately after nested item content line', () => {
    const doc = '- [[wiki]]\n\n1. a\n\t1. item\n***';
    // eslint-disable-next-line no-console
    console.log('AFTER ITEM\n' + dump(doc));
    expect(dump(doc)).toBeTruthy();
  });
});
