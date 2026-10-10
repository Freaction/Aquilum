import { describe, expect, it } from 'vitest';
import { linkedFilePath, sanitizeFileName } from '../modules/documents/documentFactory';

describe('search result note names', () => {
  it('turns a search query into a safe cross-platform filename', () => {
    expect(sanitizeFileName('  План: запуск / июль?  ')).toBe('План запуск июль');
    expect(sanitizeFileName('CON')).toBe('CON note');
  });

  it('falls back when a query has no usable filename characters', () => {
    expect(sanitizeFileName(' /:*? ')).toBe('');
  });
});

describe('linked note paths', () => {
  it('keeps valid wiki targets exact and strips a Markdown extension', () => {
    expect(linkedFilePath('C:\\notes', 'Plan')).toBe('C:\\notes\\Plan.md');
    expect(linkedFilePath('C:\\notes', 'Folder/Plan.MD')).toBe('C:\\notes\\Folder\\Plan.md');
  });

  it('rejects targets that would resolve to a different filename', () => {
    expect(linkedFilePath('C:\\notes', '../Plan')).toBeNull();
    expect(linkedFilePath('C:\\notes', 'Bad:Name')).toBeNull();
  });
});
