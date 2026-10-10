import { describe, expect, it } from 'vitest';
import { EditorState } from '@codemirror/state';
import { findBookCallouts } from './constructs';
import { parseBookCalloutBlock } from './model';

describe('bookCallout model', () => {
  it('parses inline title and fields', () => {
    const model = parseBookCalloutBlock(
      '> [!book] Чистый код\n> Автор: Роберт Мартин\n> Обложка: Files/cover.jpg',
    );
    expect(model?.title).toBe('Чистый код');
    expect(model?.wikiTarget).toBeNull();
    expect(model?.author).toBe('Роберт Мартин');
    expect(model?.cover).toBe('Files/cover.jpg');
  });

  it('parses wiki header', () => {
    const model = parseBookCalloutBlock('> [!book] [[От хорошего к великому]]');
    expect(model?.title).toBe('От хорошего к великому');
    expect(model?.wikiTarget).toBe('От хорошего к великому');
  });
});

describe('bookCallout constructs', () => {
  it('finds callout spans', () => {
    const state = EditorState.create({
      doc: 'intro\n\n> [!book] Title\n> Автор: A\n\nmore',
    });
    const spans = findBookCallouts(state.doc);
    expect(spans).toHaveLength(1);
    expect(spans[0].text).toContain('[!book] Title');
  });

});
