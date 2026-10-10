import { Prec } from '@codemirror/state';
import { linkInteraction } from './interaction';
import { markdownLinkFlow } from './flow';
import { markdownLinkMotion } from './motion';
import { autoLinkTitleExtension } from './autoTitle';

export { wikiLinkMarkdownConfig, plainBracketMarkdownConfig } from './syntax';
export { insertClipboardAsLink } from './autoTitle';

export function editorLinkExtension(
  onOpenWiki: Parameters<typeof linkInteraction>[0],
  onOpenExternal: Parameters<typeof linkInteraction>[1],
  autoLinkTitle = true,
) {
  return [
    markdownLinkFlow,
    Prec.high(markdownLinkMotion),
    Prec.high(linkInteraction(onOpenWiki, onOpenExternal)),
    autoLinkTitleExtension(autoLinkTitle),
  ];
}
