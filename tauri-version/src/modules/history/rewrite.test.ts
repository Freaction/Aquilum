import { beforeEach, describe, expect, it, vi } from 'vitest';
import { revertDocument, rewriteDocument } from '../documents/documentGateway';
import { readNoteVersion, type NoteVersion } from './index';
import { restoreVersion, revertVersion } from './rewrite';

vi.mock('../documents/documentGateway', () => ({
  rewriteDocument: vi.fn(async () => true),
  revertDocument: vi.fn(async () => true),
}));
vi.mock('./index', () => ({ readNoteVersion: vi.fn() }));

const version: NoteVersion = {
  id: '1790000000000_aaaaaaaa_agent.md',
  atMs: 1790000000000,
  device: 'aaaaaaaa',
  source: 'agent',
  fromMs: null, name: null, isCurrent: false,
};

describe('rewriting a note from its history', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(readNoteVersion).mockResolvedValue({ text: 'версия\n', previous: 'до версии\n' });
  });

  it('restores the version text as a restore write', async () => {
    expect(await restoreVersion('C:/База/Идея.md', version)).toBe(true);

    const [path, rewrite, source] = vi.mocked(rewriteDocument).mock.calls[0];
    expect(path).toBe('C:/База/Идея.md');
    expect(rewrite('что угодно')).toBe('версия\n');
    expect(source).toEqual({ kind: 'restore', fromMs: version.atMs });
  });

  it('runs once when asked twice for the same note', async () => {
    await Promise.all([restoreVersion('C:/База/Идея.md', version), restoreVersion('C:/База/Идея.md', version)]);
    expect(rewriteDocument).toHaveBeenCalledTimes(1);
  });

  it('leaves the document alone when the version is gone', async () => {
    vi.mocked(readNoteVersion).mockResolvedValueOnce(null);
    expect(await restoreVersion('C:/База/Идея.md', version)).toBe(false);
    expect(rewriteDocument).not.toHaveBeenCalled();
  });

  it('asks the document core to revert exactly that version', async () => {
    expect(await revertVersion('C:/База/Идея.md', version)).toBe(true);
    expect(revertDocument).toHaveBeenCalledWith('C:/База/Идея.md', 'версия\n', 'до версии\n', version.atMs);
  });

  it('does not revert the first version: there is nothing before it', async () => {
    vi.mocked(readNoteVersion).mockResolvedValueOnce({ text: 'первая\n', previous: null });
    expect(await revertVersion('C:/База/Идея.md', version)).toBe(false);
    expect(revertDocument).not.toHaveBeenCalled();
  });
});
