import { describe, expect, it } from 'vitest';
import { normalizeVaultRelativePath, resolveVaultAbsolutePath } from './vaultAssets';

const VAULT = 'D:/База знаний/NeuroNet';

describe('normalizeVaultRelativePath', () => {
  it('drops the current-folder prefix and resolves parents', () => {
    expect(normalizeVaultRelativePath('./Files/a.png')).toBe('Files/a.png');
    expect(normalizeVaultRelativePath('Files/../Files/a.png')).toBe('Files/a.png');
  });
});

describe('resolveVaultAbsolutePath', () => {
  it('joins a relative path to the vault', () => {
    expect(resolveVaultAbsolutePath(VAULT, 'Files/a.png')).toBe(`${VAULT}/Files/a.png`);
  });

  it('decodes percent escapes written by other editors', () => {
    expect(resolveVaultAbsolutePath(VAULT, 'Files/2026-08-09%2008-08-49.mp4'))
      .toBe(`${VAULT}/Files/2026-08-09 08-08-49.mp4`);
  });

  it('keeps a malformed escape instead of throwing', () => {
    expect(resolveVaultAbsolutePath(VAULT, 'Files/100%.png')).toBe(`${VAULT}/Files/100%.png`);
  });

  it('keeps an absolute path as it is', () => {
    expect(resolveVaultAbsolutePath(VAULT, 'C:/Other/a.png')).toBe('C:/Other/a.png');
  });

  it('unwraps a file url', () => {
    expect(resolveVaultAbsolutePath(VAULT, 'file:///C:/Other/a%20b.png')).toBe('C:/Other/a b.png');
  });
});
