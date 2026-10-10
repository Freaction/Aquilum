import { HighlightStyle, syntaxHighlighting } from '@codemirror/language';
import { tags, type Tag } from '@lezer/highlight';

interface CodeTokenStyle {
  tag: Tag[];
  class: string;
}

const CODE_TOKENS: readonly CodeTokenStyle[] = [
  {
    tag: [tags.keyword, tags.controlKeyword, tags.moduleKeyword, tags.modifier, tags.definitionKeyword, tags.operatorKeyword],
    class: 'q-code-keyword',
  },
  {
    tag: [tags.string, tags.special(tags.string), tags.regexp, tags.character],
    class: 'q-code-string',
  },
  {
    tag: [tags.number, tags.bool, tags.null, tags.atom],
    class: 'q-code-number',
  },
  {
    tag: [tags.comment, tags.lineComment, tags.blockComment, tags.docComment],
    class: 'q-code-comment',
  },
  {
    tag: [tags.function(tags.variableName), tags.function(tags.propertyName), tags.macroName],
    class: 'q-code-function',
  },
  {
    tag: [tags.typeName, tags.className, tags.namespace, tags.standard(tags.typeName), tags.tagName],
    class: 'q-code-type',
  },
  {
    tag: [tags.propertyName, tags.attributeName, tags.definition(tags.propertyName)],
    class: 'q-code-property',
  },
];

export const codeHighlighting = syntaxHighlighting(HighlightStyle.define(CODE_TOKENS.map((token) => ({ ...token }))));
