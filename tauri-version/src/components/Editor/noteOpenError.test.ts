import { describe, expect, it, vi } from 'vitest';
import { revealItemInDir } from '@tauri-apps/plugin-opener';
import { noteOpenError } from './noteOpenError';

vi.mock('@tauri-apps/plugin-opener', () => ({ revealItemInDir: vi.fn().mockResolvedValue(undefined) }));

const PATH = 'C:/vault/Заметка.md';

describe('noteOpenError', () => {
  it('offers to show a file that is not UTF-8 and keeps the plain retry', async () => {
    const error = noteOpenError(PATH, { code: 'invalid_utf8', details: { message: 'bad byte' } });

    expect(error.message).toContain('UTF-8');
    expect(error.recovery?.retriesAfter).toBe(false);
    await error.recovery?.run();
    expect(revealItemInDir).toHaveBeenCalledWith(PATH);
  });

  it('leaves only the retry when the file itself could not be read', () => {
    const error = noteOpenError(PATH, { code: 'io', details: { message: 'Отказано в доступе' } });

    expect(error.message).toContain('Отказано в доступе');
    expect(error.recovery).toBeNull();
  });
});
