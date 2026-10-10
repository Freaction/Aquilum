import { GFM, parser } from '@lezer/markdown';
import { describe, expect, it } from 'vitest';
import { plainBracketMarkdownConfig, wikiLinkMarkdownConfig } from './syntax';

describe('wiki link Markdown grammar', () => {
  const wikiParser = parser.configure(wikiLinkMarkdownConfig);

  it('parses targets and aliases as dedicated nodes', () => {
    expect(wikiParser.parse('[[Note]]').toString()).toContain(
      'WikiLink(WikiLinkMark,WikiLinkTarget,WikiLinkMark)',
    );
    expect(wikiParser.parse('[[Note|alias]]').toString()).toContain(
      'WikiLink(WikiLinkMark,WikiLinkTarget,WikiLinkAliasMark,WikiLinkAlias,WikiLinkMark)',
    );
  });
});

describe('brackets without a target', () => {
  const bracketParser = parser.configure([GFM, wikiLinkMarkdownConfig, plainBracketMarkdownConfig]);

  it('leaves a bare bracket pair as plain text', () => {
    expect(bracketParser.parse('tags: [работа, идеи]').toString()).toBe('Document(Paragraph)');
    expect(bracketParser.parse('[черновик] текст').toString()).toBe('Document(Paragraph)');
  });

  it('keeps every bracket form that names a target', () => {
    expect(bracketParser.parse('[текст](https://a.b)').toString()).toContain('Link(LinkMark,LinkMark,LinkMark,URL,LinkMark)');
    expect(bracketParser.parse('![alt](img.png)').toString()).toContain('Image(');
    expect(bracketParser.parse('[ref][id]').toString()).toContain('Link(LinkMark,LinkMark,LinkLabel)');
    expect(bracketParser.parse('[[Note]]').toString()).toContain('WikiLink(');
    expect(bracketParser.parse('- [ ] задача').toString()).toContain('Task(TaskMarker)');
  });
});
