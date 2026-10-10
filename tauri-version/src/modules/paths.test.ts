import { describe, expect, it } from 'vitest';
import { childPath, isInsidePath, rebasedPath, relativePath, samePath, siblingPath } from './paths';

describe('paths', () => {
  it('treats only real descendants as inside a folder', () => {
    expect(isInsidePath('C:\\vault\\Архив\\А.md', 'C:\\vault\\Архив')).toBe(true);
    expect(isInsidePath('C:\\vault\\Архив 2\\А.md', 'C:\\vault\\Архив')).toBe(false);
    expect(isInsidePath('C:\\vault\\Архив', 'C:\\vault\\Архив')).toBe(false);
  });

  it('moves a folder and everything under it to the new place', () => {
    expect(rebasedPath('C:\\vault\\Архив', 'C:\\vault\\Архив', 'C:\\vault\\Старое')).toBe('C:\\vault\\Старое');
    expect(rebasedPath('C:\\vault\\Архив\\Глубже', 'C:\\vault\\Архив', 'C:\\vault\\Старое'))
      .toBe('C:\\vault\\Старое\\Глубже');
    expect(rebasedPath('C:\\vault\\Архив 2', 'C:\\vault\\Архив', 'C:\\vault\\Старое')).toBeNull();
  });

  it('treats slash and case variants of one Windows path as the same path', () => {
    expect(samePath('D:\\vault\\books\\Note.md', 'D:/vault/books/Note.md')).toBe(true);
    expect(samePath('D:\\Vault\\Note.md', 'd:/vault/note.md')).toBe(true);
  });

  it('does not accept paths outside the workspace', () => {
    expect(relativePath('C:\\notes', 'C:\\notes\\folder\\note.md')).toBe('folder/note.md');
    expect(() => relativePath('C:\\notes', 'C:\\other\\note.md')).toThrow();
  });

  it('accepts equivalent Windows extended-length paths', () => {
    expect(relativePath('C:\\notes', '\\\\?\\C:\\notes\\folder\\note.md')).toBe('folder/note.md');
    expect(relativePath('\\\\?\\C:\\notes', 'C:\\notes\\folder\\note.md')).toBe('folder/note.md');
  });

  it('joins names with the separator the directory already uses', () => {
    expect(childPath('C:\\notes\\', 'Plan.md')).toBe('C:\\notes\\Plan.md');
    expect(childPath('/notes', 'Plan.md')).toBe('/notes/Plan.md');
    expect(siblingPath('C:\\notes\\Old.md', 'New.md')).toBe('C:\\notes\\New.md');
  });
});
