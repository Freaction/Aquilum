type SyntaxMarkerPresentation = 'hidden' | 'collapsed' | 'transparent' | 'horizontal-rule';

const syntaxMarkerPresentations: Readonly<Record<string, SyntaxMarkerPresentation>> = {
  EmphasisMark: 'hidden',
  StrongMark: 'hidden',
  StrikethroughMark: 'hidden',
  HeaderMark: 'hidden',
  LinkMark: 'collapsed',
  URL: 'collapsed',
  CodeMark: 'hidden',
  QuoteMark: 'transparent',
  WikiLinkMark: 'collapsed',
  WikiLinkAliasMark: 'collapsed',
  HorizontalRule: 'horizontal-rule',
};

export function syntaxMarkerPresentation(nodeName: string): SyntaxMarkerPresentation | null {
  return syntaxMarkerPresentations[nodeName] ?? null;
}

