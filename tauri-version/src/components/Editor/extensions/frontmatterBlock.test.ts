import { describe, expect, it } from 'vitest';
import { markdown, markdownLanguage } from '@codemirror/lang-markdown';
import { EditorState } from '@codemirror/state';
import { frontmatterBlock } from './frontmatterBlock';
import {
  isFrontmatterExpanded,
  setFrontmatterExpanded,
} from './frontmatterUi';
import { frontmatterRange } from '../../../modules/docs/frontmatter';
import { outlineMarkdownConfig } from './outline';
import { wikiLinkMarkdownConfig } from './links';

const FM_DOC = [
  '---',
  'cover: true',
  'type: book',
  '---',
  '',
  '# Body',
].join('\n');

function createState(doc = FM_DOC): EditorState {
  return EditorState.create({
    doc,
    extensions: [
      markdown({
        base: markdownLanguage,
        extensions: [outlineMarkdownConfig, wikiLinkMarkdownConfig],
      }),
      frontmatterBlock,
    ],
  });
}

describe('frontmatter collapse', () => {
  it('stays expanded across doc edits', () => {
    let state = createState();
    state = state.update({ effects: setFrontmatterExpanded.of(true) }).state;
    expect(isFrontmatterExpanded(state)).toBe(true);

    state = state.update({
      changes: { from: state.doc.line(1).to - 1, to: state.doc.line(1).to, insert: '' },
    }).state;
    expect(state.doc.line(1).text).toBe('--');
    expect(frontmatterRange(state.doc.toString())).toBeNull();
    expect(isFrontmatterExpanded(state)).toBe(true);
  });

  it('toggles only via effect', () => {
    let state = createState();
    expect(isFrontmatterExpanded(state)).toBe(false);
    state = state.update({ effects: setFrontmatterExpanded.of(true) }).state;
    expect(isFrontmatterExpanded(state)).toBe(true);
  });

  it('expands frontmatter created during editing', () => {
    let state = createState('---\ncover: true\n--');
    state = state.update({
      changes: { from: state.doc.length, insert: '-' },
    }).state;
    expect(frontmatterRange(state.doc.toString())).not.toBeNull();
    expect(isFrontmatterExpanded(state)).toBe(true);
  });

  it('keeps the blank line after newly created frontmatter', () => {
    let state = createState('---\ncover: true\n--\n\nBody');
    state = state.update({
      changes: { from: state.doc.line(3).to, insert: '-' },
    }).state;
    expect(state.doc.toString()).toBe('---\ncover: true\n---\n\nBody');
    expect(isFrontmatterExpanded(state)).toBe(true);
  });
});
