import { syntaxTree } from '@codemirror/language';
import {
  Decoration,
  type DecorationSet,
  EditorView,
  ViewPlugin,
  type ViewUpdate,
} from '@codemirror/view';
import type { SyntaxNode } from '@lezer/common';
import { ensureEditorTree, visibleTreeRanges } from '../ensureEditorTree';

const linkFlow = Decoration.mark({ class: 'q-md-link-flow' });

export function linkLabelRange(node: SyntaxNode): { from: number; to: number } | null {
  const cursor = node.cursor();
  if (!cursor.firstChild()) return null;
  let labelFrom: number | null = null;
  let labelTo: number | null = null;
  do {
    if (cursor.name === 'URL') break;
    if (cursor.name !== 'LinkMark') continue;
    if (labelFrom === null) {
      labelFrom = cursor.to;
    } else {
      labelTo = cursor.from;
      break;
    }
  } while (cursor.nextSibling());
  if (labelFrom === null || labelTo === null || labelTo <= labelFrom) return null;
  return { from: labelFrom, to: labelTo };
}

export function linkUrlChromeRange(node: SyntaxNode): { from: number; to: number } | null {
  const label = linkLabelRange(node);
  if (!label || label.to >= node.to) return null;
  return { from: label.to, to: node.to };
}

function buildLinkFlow(view: EditorView, allowParse = true): DecorationSet {
  const ranges: { from: number; to: number }[] = [];
  const tree = ensureEditorTree(view, allowParse);
  const head = view.state.selection.main.head;

  for (const range of visibleTreeRanges(view, tree.length)) {
    tree.iterate({
      from: range.from,
      to: Math.min(range.to, tree.length),
      enter(node) {
        if (node.name !== 'Link' && node.name !== 'Image') return;
        const label = linkLabelRange(node.node);
        if (label) ranges.push(label);
        if (head >= node.from && head <= node.to) {
          const chrome = linkUrlChromeRange(node.node);
          if (chrome) ranges.push(chrome);
        }
      },
    });
  }

  return Decoration.set(
    ranges
      .sort((a, b) => a.from - b.from || a.to - b.to)
      .map((range) => linkFlow.range(range.from, range.to)),
  );
}

export const markdownLinkFlow = ViewPlugin.fromClass(class {
  decorations: DecorationSet;

  constructor(view: EditorView) {
    this.decorations = buildLinkFlow(view);
  }

  update(update: ViewUpdate) {
    const treeChanged = syntaxTree(update.state) !== syntaxTree(update.startState);
    if (update.docChanged || update.viewportChanged || update.selectionSet || treeChanged) {
      this.decorations = buildLinkFlow(update.view, update.docChanged || treeChanged);
    }
  }
}, { decorations: (value) => value.decorations });
