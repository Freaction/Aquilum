import { EditorState, StateEffect, StateField } from '@codemirror/state';
import { frontmatterRange } from '../../../modules/docs/frontmatter';

export const setFrontmatterExpanded = StateEffect.define<boolean>();

const FRONTMATTER_HEAD_LIMIT = 1500;

export function stateFrontmatterRange(state: EditorState): { from: number; to: number } | null {
  return frontmatterRange(state.doc.sliceString(0, Math.min(FRONTMATTER_HEAD_LIMIT, state.doc.length)));
}

function hasFrontmatter(state: EditorState): boolean {
  return stateFrontmatterRange(state) !== null;
}

export const frontmatterCollapseField = StateField.define<{ expanded: boolean }>({
  create: () => ({ expanded: false }),
  update(value, tr) {
    let expanded = value.expanded;
    if (
      tr.docChanged
      && !hasFrontmatter(tr.startState)
      && hasFrontmatter(tr.state)
    ) {
      expanded = true;
    }
    for (const effect of tr.effects) {
      if (effect.is(setFrontmatterExpanded)) expanded = effect.value;
    }
    return { expanded };
  },
});

export function isFrontmatterExpanded(state: EditorState): boolean {
  return state.field(frontmatterCollapseField).expanded;
}
