import { describe, expect, it, vi } from 'vitest';
import { applyRelocation } from './relocation';

describe('applyRelocation', () => {
  it('moves the tabs of renamed notes and closes the tabs of removed ones', () => {
    const renamed = vi.fn();
    const deleted = vi.fn();

    applyRelocation(
      { moves: [{ from: 'C:/База/Старое.md', to: 'C:/База/Новое.md' }], removed: ['C:/База/Удалено.md'] },
      { renamed, deleted },
    );

    expect(renamed).toHaveBeenCalledWith('C:/База/Старое.md', 'C:/База/Новое.md');
    expect(deleted).toHaveBeenCalledWith('C:/База/Удалено.md');
  });
});
