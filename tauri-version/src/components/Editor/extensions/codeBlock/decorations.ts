import { Facet, RangeSetBuilder, type Text } from '@codemirror/state';
import { Decoration, type DecorationSet } from '@codemirror/view';
import type { CodeBlockShape } from './blocks';

// Code Styler использует тот же генератор для видимых строк и заменяет базовый слой.
export const codeBlockLineDecorationsFacet = Facet.define<boolean, boolean>({
  combine: values => values.every(Boolean),
});

const LINE_CLASS = 'q-md-code-line';

function lineDecoration(...modifiers: readonly string[]): Decoration {
  return Decoration.line({ class: [LINE_CLASS, ...modifiers].join(' ') });
}

const BODY_LINE = lineDecoration();
const FIRST_LINE = lineDecoration(`${LINE_CLASS}--first`);
const LAST_LINE = lineDecoration(`${LINE_CLASS}--last`);
const ONLY_LINE = lineDecoration(`${LINE_CLASS}--first`, `${LINE_CLASS}--last`);

function edgeDecoration(isFirst: boolean, isLast: boolean): Decoration {
  if (isFirst && isLast) return ONLY_LINE;
  if (isFirst) return FIRST_LINE;
  if (isLast) return LAST_LINE;
  return BODY_LINE;
}

export function codeBlockDecorations(doc: Text, blocks: readonly CodeBlockShape[]): DecorationSet {
  const builder = new RangeSetBuilder<Decoration>();

  for (const block of blocks) {
    const first = doc.lineAt(block.from).number;
    const last = doc.lineAt(block.end).number;
    for (let number = first; number <= last; number += 1) {
      const { from } = doc.line(number);
      builder.add(from, from, edgeDecoration(number === first, number === last));
    }
  }

  return builder.finish();
}
