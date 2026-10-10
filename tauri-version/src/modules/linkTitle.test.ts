import { describe, expect, it } from 'vitest';
import {
  escapeMarkdownTitle,
  hostnameFallback,
  isBareHttpUrl,
  isImageUrl,
  isMarkdownLinkContext,
  makeTitlePlaceholder,
  markdownLink,
  normalizeFetchUrl,
} from './linkTitle';

describe('linkTitle helpers', () => {
  it('detects bare http urls and rejects multiline', () => {
    expect(isBareHttpUrl('https://example.com/path')).toBe(true);
    expect(isBareHttpUrl('www.example.com')).toBe(true);
    expect(isBareHttpUrl('https://example.com\nmore')).toBe(false);
    expect(isBareHttpUrl('not a url')).toBe(false);
  });

  it('detects image urls', () => {
    expect(isImageUrl('https://cdn.example.com/a.png')).toBe(true);
    expect(isImageUrl('https://cdn.example.com/a.png?w=1')).toBe(true);
    expect(isImageUrl('https://example.com/page')).toBe(false);
  });

  it('detects markdown link context before cursor', () => {
    expect(isMarkdownLinkContext('see ](')).toBe(true);
    expect(isMarkdownLinkContext('[label](https://ex')).toBe(true);
    expect(isMarkdownLinkContext('plain text')).toBe(false);
  });

  it('escapes markdown title metacharacters', () => {
    expect(escapeMarkdownTitle('A *B* [C]')).toBe('A \\*B\\* \\[C\\]');
  });

  it('builds placeholder ids and markdown links', () => {
    const a = makeTitlePlaceholder();
    const b = makeTitlePlaceholder();
    expect(a).toMatch(/^Fetching Title#/);
    expect(a).not.toBe(b);
    expect(markdownLink('Title', 'https://x.test')).toBe('[Title](https://x.test)');
  });

  it('normalizes www and hostname fallback', () => {
    expect(normalizeFetchUrl('www.example.com')).toBe('https://www.example.com');
    expect(hostnameFallback('https://example.com/path')).toBe('example.com');
  });
});
