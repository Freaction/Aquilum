import { describe, expect, it } from 'vitest';
import { EditorState } from '@codemirror/state';
import { frontmatterRange, noteFrontmatterTemplate } from '../../../modules/docs/frontmatter';
import {
  canInsertFrontmatter,
  frontmatterCaretOffset,
  frontmatterInsertText,
} from './frontmatterInsert';
import { frontmatterCollapseField, isFrontmatterExpanded } from './frontmatterUi';

const TEMPLATE = noteFrontmatterTemplate();

function stateWith(doc: string): EditorState {
  return EditorState.create({ doc, extensions: [frontmatterCollapseField] });
}

describe('frontmatter template', () => {
  it('содержит запрошенные поля и разбирается как frontmatter', () => {
    expect(TEMPLATE).toBe('---\ntags: []\nauthor: \nsource: \n---');
    expect(frontmatterRange(TEMPLATE)).not.toBeNull();
  });

  it('ставит каретку между скобками tags', () => {
    const offset = frontmatterCaretOffset(TEMPLATE);
    expect(TEMPLATE.slice(offset - 1, offset + 1)).toBe('[]');
  });

  it('переводит строку только перед непустым документом', () => {
    expect(frontmatterInsertText(stateWith(''), TEMPLATE)).toBe(TEMPLATE);
    expect(frontmatterInsertText(stateWith('Текст'), TEMPLATE)).toBe(`${TEMPLATE}\n`);
  });
});

describe('frontmatter insertion guard', () => {
  it('разрешает вставку в пустую заметку и в заметку без метаданных', () => {
    expect(canInsertFrontmatter(stateWith(''))).toBe(true);
    expect(canInsertFrontmatter(stateWith('Просто текст\n\nи ещё'))).toBe(true);
  });

  it('запрещает вставку, когда метаданные уже есть', () => {
    expect(canInsertFrontmatter(stateWith(`${TEMPLATE}\nТекст`))).toBe(false);
  });

  it('не считает метаданными блок, начинающийся не с первой строки', () => {
    expect(canInsertFrontmatter(stateWith(`Текст\n${TEMPLATE}\n`))).toBe(true);
  });
});

describe('frontmatter collapse state', () => {
  it('раскрывается сам, когда метаданные появились в документе', () => {
    const before = stateWith('Текст');
    expect(isFrontmatterExpanded(before)).toBe(false);

    const after = before.update({
      changes: { from: 0, insert: frontmatterInsertText(before, TEMPLATE) },
    }).state;

    expect(isFrontmatterExpanded(after)).toBe(true);
  });
});
