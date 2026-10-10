import type { SyntaxNode } from '@lezer/common';
import { findAncestor } from './syntaxAncestor';

interface RevealTarget {
  from: number;
  to: number;
  hideTo: number;
}

const HEADING_OWNERS = [
  'ATXHeading1',
  'ATXHeading2',
  'ATXHeading3',
  'ATXHeading4',
  'ATXHeading5',
  'ATXHeading6',
  'SetextHeading1',
  'SetextHeading2',
];
const BLOCKQUOTE_OWNERS = ['Blockquote'];
export const LINK_OWNERS: readonly string[] = ['WikiLink', 'Link', 'Image'];
const EMPHASIS_OWNERS = ['Emphasis', 'StrongEmphasis', 'Strikethrough'];

const markerOwners: Readonly<Record<string, readonly string[]>> = {
  HeaderMark: HEADING_OWNERS,
  QuoteMark: BLOCKQUOTE_OWNERS,
  LinkMark: LINK_OWNERS,
  URL: LINK_OWNERS,
  WikiLinkMark: LINK_OWNERS,
  WikiLinkAliasMark: LINK_OWNERS,
  EmphasisMark: EMPHASIS_OWNERS,
  StrongMark: EMPHASIS_OWNERS,
  StrikethroughMark: EMPHASIS_OWNERS,
};

function nearestOwner(node: SyntaxNode): SyntaxNode | null {
  const owners = markerOwners[node.name];
  if (!owners) return null;
  return findAncestor(node.parent, (candidate) => owners.includes(candidate.name));
}

function hiddenTo(node: SyntaxNode, charAt: (pos: number) => string): number {
  const eatsTrailingSpace = node.name === 'HeaderMark' && charAt(node.to) === ' ';
  return eatsTrailingSpace ? node.to + 1 : node.to;
}

export function revealTarget(
  node: SyntaxNode,
  charAt: (pos: number) => string,
): RevealTarget {
  const hideTo = hiddenTo(node, charAt);
  const owner = nearestOwner(node) ?? node;
  return { from: owner.from, to: Math.max(owner.to, hideTo), hideTo };
}
