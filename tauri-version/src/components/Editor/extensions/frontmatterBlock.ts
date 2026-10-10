import { type EditorState, Prec, StateField } from '@codemirror/state';
import { Decoration, type DecorationSet, EditorView } from '@codemirror/view';
import {
  frontmatterCollapseField,
  setFrontmatterExpanded,
  stateFrontmatterRange,
} from './frontmatterUi';

const frontmatterMark = Decoration.mark({ class: 'q-cm-frontmatter' });
const frontmatterKeyMark = Decoration.mark({ class: 'q-cm-frontmatter-key' });
const frontmatterValueMark = Decoration.mark({ class: 'q-cm-frontmatter-value' });

function styleFrontmatter(state: EditorState, range: { from: number; to: number }): DecorationSet {
  const decorations = [];
  const start = state.doc.lineAt(range.from).number;
  const end = state.doc.lineAt(range.to).number;

  for (let n = start; n <= end; n++) {
    const line = state.doc.line(n);
    const text = line.text;
    if (!text.trim()) continue;

    decorations.push(frontmatterMark.range(line.from, line.to));
    const colon = text.indexOf(':');
    if (colon === -1) continue;

    decorations.push(frontmatterKeyMark.range(line.from, line.from + colon + 1));
    if (line.to > line.from + colon + 1) {
      decorations.push(frontmatterValueMark.range(line.from + colon + 1, line.to));
    }
  }

  decorations.sort((a, b) => a.from - b.from || b.to - a.to);
  return Decoration.set(decorations);
}

function buildDecorations(state: EditorState): DecorationSet {
  const range = stateFrontmatterRange(state);
  if (!range) return Decoration.none;

  if (!state.field(frontmatterCollapseField).expanded) {
    return Decoration.set([
      Decoration.replace({ block: true }).range(range.from, range.to),
    ]);
  }

  return styleFrontmatter(state, range);
}

export const frontmatterBlock = [
  frontmatterCollapseField,
  Prec.highest(
    StateField.define<DecorationSet>({
      create: buildDecorations,
      update(deco, tr) {
        if (tr.docChanged || tr.effects.some((e) => e.is(setFrontmatterExpanded))) {
          return buildDecorations(tr.state);
        }
        return deco.map(tr.changes);
      },
      provide: (field) => EditorView.decorations.from(field),
    }),
  ),
];
