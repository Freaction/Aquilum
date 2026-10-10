import { describe, expect, it } from 'vitest';
import { markdown, markdownLanguage } from '@codemirror/lang-markdown';
import { EditorSelection, EditorState } from '@codemirror/state';
import { outlineMarkdownConfig } from '../outline';
import { wikiLinkMarkdownConfig } from './syntax';
import { editablePreviewBlockEntryPos, selectionInEditablePreviewBlock } from '../livePreviewEditableSpans';
import { findReaderQuotes, readerQuoteEditEntryPos } from '../readerQuote/constructs';
import { docLineStepPos, inlineLinkOwnerAt } from './motion';

function createState(doc: string, head: number) {
  return EditorState.create({
    doc,
    selection: EditorSelection.cursor(head),
    extensions: [
      markdown({
        base: markdownLanguage,
        extensions: [outlineMarkdownConfig, wikiLinkMarkdownConfig],
      }),
    ],
  });
}

describe('inlineLinkOwnerAt', () => {
  const doc = '- [label](https://example.com/long)\nnext\n';
  const label = doc.indexOf('label');
  const linkEnd = doc.indexOf(')') + 1;

  it('finds the link while caret is in the label', () => {
    const state = createState(doc, label);
    expect(inlineLinkOwnerAt(state, label)).toEqual({
      from: doc.indexOf('['),
      to: linkEnd,
    });
  });

  it('keeps the zone at caret-after-last-char', () => {
    const state = createState(doc, linkEnd);
    expect(inlineLinkOwnerAt(state, linkEnd)?.to).toBe(linkEnd);
  });

  it('returns null on the next line', () => {
    const state = createState(doc, doc.indexOf('next'));
    expect(inlineLinkOwnerAt(state, doc.indexOf('next'))).toBeNull();
  });
});

describe('docLineStepPos', () => {
  const doc = 'line one with [link](https://example.com)\nshort\n';

  it('steps to the next document line preserving column (clamped)', () => {
    const head = doc.indexOf('link');
    const state = createState(doc, head);
    const col = head - state.doc.lineAt(head).from;
    const next = state.doc.line(2);
    expect(docLineStepPos(state, head, true)).toBe(Math.min(next.from + col, next.to));
  });

  it('returns null on the last document line', () => {
    const state = createState('only line', 0);
    expect(docLineStepPos(state, 0, true)).toBeNull();
  });
});

describe('selection in quote edit zone', () => {
  const doc = '> [!quote] Quote text [1](aquilum-reader:cfi=x&book=files%2Fvery%2Flong%2Fpath%2Fto%2Fbook.epub)\nafter\n';

  it('treats caret in ref link as inside the preview block', () => {
    const state = createState(doc, doc.lastIndexOf(')'));
    expect(selectionInEditablePreviewBlock(state)).toBe(true);
  });
});

describe('editablePreviewBlockEntryPos (quote navigation)', () => {
  const doc = 'before\n> [!quote] Quote [1](aquilum-reader:cfi=x)\nafter\n';

  it('targets quote text end when arrow-up crosses from below', () => {
    const state = createState(doc, doc.indexOf('after'));
    const pos = editablePreviewBlockEntryPos(state, 3, false);
    const span = findReaderQuotes(state.doc)[0]!;
    expect(pos).toBe(readerQuoteEditEntryPos(span));
  });
});
