import { describe, expect, it } from 'vitest';
import { separatePastedFrontmatter } from './frontmatterPaste';

describe('separatePastedFrontmatter', () => {
  it('adds an empty line between frontmatter and pasted content', () => {
    expect(separatePastedFrontmatter('---\ntags: []\n---\nBody'))
      .toBe('---\ntags: []\n---\n\nBody');
  });

  it('adds an empty line after a standalone frontmatter block', () => {
    expect(separatePastedFrontmatter('---\r\ntags: []\r\n---'))
      .toBe('---\r\ntags: []\r\n---\r\n\r\n');
  });

  it('keeps already separated and ordinary clipboard text unchanged', () => {
    expect(separatePastedFrontmatter('---\ntags: []\n---\n\nBody'))
      .toBe('---\ntags: []\n---\n\nBody');
    expect(separatePastedFrontmatter('Body\n---')).toBe('Body\n---');
  });
});
