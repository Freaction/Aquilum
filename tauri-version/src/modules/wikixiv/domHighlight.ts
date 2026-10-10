import { getWikiHover, subscribeWikiHover } from './hoverHighlight';
import { wikiTermMatcher } from './termMatcher';

function clearHighlights(element: HTMLElement): void {
  element.querySelectorAll<HTMLElement>('.q-wiki-hover-highlight').forEach((highlight) => {
    highlight.replaceWith(...Array.from(highlight.childNodes));
  });
  element.normalize();
}

function highlightText(element: HTMLElement, matcher: RegExp): void {
  const walker = document.createTreeWalker(element, NodeFilter.SHOW_TEXT);
  const nodes: Text[] = [];
  while (walker.nextNode()) nodes.push(walker.currentNode as Text);

  for (const node of nodes) {
    const text = node.data;
    matcher.lastIndex = 0;
    const matches = Array.from(text.matchAll(matcher));
    if (!matches.length) continue;

    const fragment = document.createDocumentFragment();
    let offset = 0;
    for (const match of matches) {
      const index = match.index ?? 0;
      fragment.append(text.slice(offset, index));
      const highlight = document.createElement('span');
      highlight.className = 'q-wiki-hover-highlight';
      highlight.textContent = match[0];
      fragment.append(highlight);
      offset = index + match[0].length;
    }
    fragment.append(text.slice(offset));
    node.replaceWith(fragment);
  }
}

const bindings = new WeakMap<HTMLElement, () => void>();
const cleanups = new WeakMap<HTMLElement, () => void>();

export function bindWikiHover(root: HTMLElement, selector: string): () => void {
  const apply = () => {
    const matcher = wikiTermMatcher(getWikiHover().terms);
    root.querySelectorAll<HTMLElement>(selector).forEach((element) => {
      clearHighlights(element);
      if (matcher) highlightText(element, matcher);
    });
  };
  cleanups.get(root)?.();
  bindings.set(root, apply);
  apply();
  const unsubscribe = subscribeWikiHover(apply);
  const cleanup = () => {
    unsubscribe();
    bindings.delete(root);
    cleanups.delete(root);
  };
  cleanups.set(root, cleanup);
  return cleanup;
}

export function refreshWikiHover(root: HTMLElement): void {
  bindings.get(root)?.();
}

export function unbindWikiHover(root: HTMLElement): void {
  cleanups.get(root)?.();
}

