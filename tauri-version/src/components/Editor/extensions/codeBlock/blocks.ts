import type { Text } from '@codemirror/state';
import type { SyntaxNode, SyntaxNodeRef, Tree } from '@lezer/common';

const BLOCK_NODES = ['FencedCode', 'CodeBlock'];

export interface CodeBlockShape {
  from: number;
  end: number;
  info: string;
}

function fenceInfo(node: SyntaxNodeRef, doc: Text): string {
  const info = node.node.getChild('CodeInfo');
  return info ? doc.sliceString(info.from, info.to).trim() : '';
}

export function codeBlocks(
  tree: Tree,
  doc: Text,
  ranges: readonly { from: number; to: number }[],
): CodeBlockShape[] {
  const blocks: CodeBlockShape[] = [];
  const seen = new Set<number>();

  for (const { from, to } of ranges) {
    tree.iterate({
      from,
      to: Math.min(to, tree.length),
      enter(node) {
        if (!BLOCK_NODES.includes(node.name)) return;
        const start = doc.lineAt(node.from).from;
        if (seen.has(start)) return;
        seen.add(start);
        const lastLine = doc.lineAt(Math.max(node.from, Math.min(node.to, doc.length) - 1));
        blocks.push({ from: start, end: lastLine.to, info: fenceInfo(node, doc) });
      },
    });
  }

  return blocks.sort((a, b) => a.from - b.from);
}

export function isFenceMark(node: { name: string; node: SyntaxNode }): boolean {
  return node.name === 'CodeMark' && node.node.parent?.name === 'FencedCode';
}
