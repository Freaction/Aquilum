import type { SyntaxNode } from '@lezer/common';

export function findAncestor(
  node: SyntaxNode | null,
  matches: (candidate: SyntaxNode) => boolean,
): SyntaxNode | null {
  for (let current = node; current; current = current.parent) {
    if (matches(current)) return current;
  }
  return null;
}

export function hasAncestorNamed(node: SyntaxNode | null, name: string): boolean {
  return findAncestor(node, (candidate) => candidate.name === name) !== null;
}
