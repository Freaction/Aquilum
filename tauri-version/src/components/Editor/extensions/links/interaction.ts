import { syntaxTree } from '@codemirror/language';
import type { EditorState } from '@codemirror/state';
import { EditorView } from '@codemirror/view';
import type { SyntaxNode } from '@lezer/common';
import type { LinkDisposition } from '../../../../modules/links';
import { READER_LINK_SCHEME } from '../../../../modules/docs/bookQuotes';
import { linkLabelRange } from './flow';
import { findAncestor } from '../syntaxAncestor';

function wikiLinkTargetAt(state: EditorState, position: number): string | null {
  const wikiLink = findAncestor(
    syntaxTree(state).resolveInner(position, -1),
    (node) => node.name === 'WikiLink',
  );
  const target = wikiLink?.getChild('WikiLinkTarget');
  return target ? state.sliceDoc(target.from, target.to).trim() : null;
}

function urlNodeOf(node: SyntaxNode): SyntaxNode | null {
  if (node.name === 'URL') return node;
  return node.name === 'Link' ? node.getChild('URL') : null;
}

function externalUrlAt(state: EditorState, position: number): string | null {
  const owner = findAncestor(
    syntaxTree(state).resolveInner(position, -1),
    (node) => urlNodeOf(node) !== null,
  );
  const url = owner && urlNodeOf(owner);
  return url ? state.sliceDoc(url.from, url.to) : null;
}

function linkNodeAt(state: EditorState, pos: number): SyntaxNode | null {
  return findAncestor(syntaxTree(state).resolveInner(pos, 1), (node) => node.name === 'Link');
}

function linkLabelAt(
  state: EditorState,
  pos: number,
): { from: number; to: number } | null {
  const link = linkNodeAt(state, pos);
  if (!link) return null;
  const label = linkLabelRange(link);
  if (!label || pos < label.from || pos > label.to) return null;
  return label;
}

export function readerRefLinkClickAllowed(
  state: EditorState,
  clickPos: number,
  url: string,
): boolean {
  if (!url.startsWith(READER_LINK_SCHEME)) return true;
  const head = state.selection.main.head;
  const labelClick = linkLabelAt(state, clickPos);
  const labelCaret = linkLabelAt(state, head);
  if (!labelClick || !labelCaret) return false;
  return labelClick.from === labelCaret.from && labelClick.to === labelCaret.to;
}

export function linkInteraction(
  onOpenWiki: (target: string, disposition: LinkDisposition) => void,
  onOpenExternal: (url: string) => void,
) {
  const activateLink = (event: MouseEvent, view: EditorView): boolean => {
    if (event.button !== 0) return false;
    const target = event.target as HTMLElement;
    if (!target.closest?.('.q-md-link')) return false;

    const pos = view.posAtCoords({ x: event.clientX, y: event.clientY });
    if (pos == null) return false;

    const wikiTarget = wikiLinkTargetAt(view.state, pos);
    if (wikiTarget) {
      event.preventDefault();
      event.stopPropagation();
      onOpenWiki(wikiTarget, event.ctrlKey || event.metaKey ? 'new-tab' : 'current');
      return true;
    }

    const url = externalUrlAt(view.state, pos);
    if (!url) return false;
    if (!readerRefLinkClickAllowed(view.state, pos, url)) return false;
    event.preventDefault();
    event.stopPropagation();
    onOpenExternal(url);
    return true;
  };

  return EditorView.domEventHandlers({
    mousedown(event, view) {
      return activateLink(event, view);
    },
  });
}
