import { beforeEach, describe, expect, it, vi } from 'vitest';
import { rewriteDocument } from './documentGateway';
import { EDIT_WRITE } from './fileGateway';
import { toggleTaskInFile } from './toggleTaskInFile';

vi.mock('./documentGateway', () => ({ rewriteDocument: vi.fn() }));

let text = '';

describe('toggleTaskInFile', () => {
  beforeEach(() => {
    vi.mocked(rewriteDocument).mockReset();
    vi.mocked(rewriteDocument).mockImplementation(async (_path, rewrite) => {
      await Promise.resolve();
      const next = rewrite(text);
      if (next === null || next === text) return false;
      text = next;
      return true;
    });
  });

  it('toggles the task through the document as the user edit it is', async () => {
    text = '- [ ] задача\n';

    expect(await toggleTaskInFile('C:/vault', 'Note.md', 1)).toBe(true);

    expect(text).toBe('- [x] задача\n');
    expect(vi.mocked(rewriteDocument).mock.calls[0][0]).toBe('C:/vault/Note.md');
    expect(vi.mocked(rewriteDocument).mock.calls[0][2]).toEqual(EDIT_WRITE);
  });

  it('leaves a line that is not a task alone', async () => {
    text = 'просто текст\n';
    expect(await toggleTaskInFile('C:/vault', 'Note.md', 1)).toBe(false);
    expect(text).toBe('просто текст\n');
  });

  it('applies concurrent toggles of one note one after another', async () => {
    text = '- [ ] первая\n- [ ] вторая\n';

    await Promise.all([
      toggleTaskInFile('C:/vault', 'Note.md', 1),
      toggleTaskInFile('C:/vault', 'Note.md', 2),
    ]);

    expect(text).toBe('- [x] первая\n- [x] вторая\n');
  });
});
