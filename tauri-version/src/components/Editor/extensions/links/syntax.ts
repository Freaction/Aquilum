import type { MarkdownConfig } from '@lezer/markdown';
import { tags } from '@lezer/highlight';

export const wikiLinkMarkdownConfig: MarkdownConfig = {
  defineNodes: [
    { name: 'WikiLink' },
    { name: 'WikiLinkMark', style: tags.processingInstruction },
    { name: 'WikiLinkTarget', style: tags.link },
    { name: 'WikiLinkAliasMark', style: tags.processingInstruction },
    { name: 'WikiLinkAlias', style: tags.link },
  ],
  parseInline: [{
    name: 'WikiLink',
    before: 'Link',
    parse(context, next, position) {
      if (next !== 91 || context.char(position + 1) !== 91) return -1;
      let close = position + 2;
      while (close + 1 < context.end) {
        const character = context.char(close);
        if (character === 10 || character === 13) return -1;
        if (character === 93 && context.char(close + 1) === 93) break;
        close += 1;
      }
      if (close + 1 >= context.end) return -1;
      let separator = -1;
      for (let cursor = position + 2; cursor < close; cursor += 1) {
        if (context.char(cursor) === 124) {
          separator = cursor;
          break;
        }
      }
      const targetEnd = separator < 0 ? close : separator;
      const children = [
        context.elt('WikiLinkMark', position, position + 2),
        context.elt('WikiLinkTarget', position + 2, targetEnd),
      ];
      if (separator >= 0) {
        children.push(context.elt('WikiLinkAliasMark', separator, separator + 1));
        children.push(context.elt('WikiLinkAlias', separator + 1, close));
      }
      children.push(context.elt('WikiLinkMark', close, close + 2));
      return context.addElement(context.elt('WikiLink', position, close + 2, children));
    },
  }],
};

const CLOSING_BRACKET = 93;
const OPENING_BRACKET = 91;
const OPENING_PAREN = 40;

export const plainBracketMarkdownConfig: MarkdownConfig = {
  parseInline: [{
    name: 'AquilumPlainBracket',
    before: 'Link',
    parse(context, next, position) {
      if (next !== CLOSING_BRACKET) return -1;
      const after = context.char(position + 1);
      if (after === OPENING_PAREN || after === OPENING_BRACKET || after === CLOSING_BRACKET) return -1;
      return position + 1;
    },
  }],
};
