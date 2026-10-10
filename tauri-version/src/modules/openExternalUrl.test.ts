import { describe, expect, it } from 'vitest';
import { normalizeExternalUrl } from './openExternalUrl';

describe('normalizeExternalUrl', () => {
  it('accepts http(s) and mailto links', () => {
    expect(normalizeExternalUrl('https://example.com')).toBe('https://example.com');
    expect(normalizeExternalUrl('<mailto:test@example.com>')).toBe('mailto:test@example.com');
  });

  it('rejects relative and empty values', () => {
    expect(normalizeExternalUrl('./note.md')).toBeNull();
    expect(normalizeExternalUrl('')).toBeNull();
  });
});
