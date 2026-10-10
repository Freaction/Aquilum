import { describe, expect, it } from 'vitest';
import { frontmatterRange, parseFrontmatter } from '../docs/frontmatter';
import { bookStarterTemplate, noteStarterTemplate } from './starter';

describe('starter templates', () => {
  it('book template keeps cover and type for layout', () => {
    const parsed = parseFrontmatter(bookStarterTemplate);
    expect(parsed?.data.cover).toBe('true');
    expect(parsed?.data.type).toBe('book');
    expect(parsed?.data.tags).toEqual(['книга']);
    expect(frontmatterRange(bookStarterTemplate)).not.toBeNull();
  });

  it('note template carries the fields a note starts with', () => {
    const parsed = parseFrontmatter(noteStarterTemplate);
    expect(parsed?.data.author).toBe('');
    expect(parsed?.data.source).toBe('');
    expect(frontmatterRange(noteStarterTemplate)).not.toBeNull();
  });
});
