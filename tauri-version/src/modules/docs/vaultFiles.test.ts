import { describe, expect, it } from 'vitest';
import { normalizeFilesFolder } from './vaultFiles';

describe('normalizeFilesFolder', () => {
  it('keeps a folder relative to the vault with forward slashes', () => {
    expect(normalizeFilesFolder('assets')).toBe('assets');
    expect(normalizeFilesFolder(' \\media\\images\\ ')).toBe('media/images');
  });

  it('falls back to Files when nothing usable is left', () => {
    expect(normalizeFilesFolder('')).toBe('Files');
    expect(normalizeFilesFolder('../..')).toBe('Files');
  });

  it('never leaves the vault', () => {
    expect(normalizeFilesFolder('C:\\Users\\pics')).toBe('Users/pics');
    expect(normalizeFilesFolder('/../assets/./img')).toBe('assets/img');
  });
});
