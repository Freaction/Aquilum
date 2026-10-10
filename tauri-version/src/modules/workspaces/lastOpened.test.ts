import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { lastOpenedWorkspace } from './index';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

const invokeMock = vi.mocked(invoke);

function known(paths: string[]) {
  return paths.map((path, index) => ({
    id: `id-${index}`,
    path,
    lastSeenMs: 1000 - index,
  }));
}

describe('lastOpenedWorkspace', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('restores the workspace the app opened last, whatever the build was', async () => {
    invokeMock.mockResolvedValue(known(['D:/NeuroNet', 'D:/knowledge base']));
    await expect(lastOpenedWorkspace()).resolves.toBe('D:/NeuroNet');
    expect(invokeMock).toHaveBeenCalledWith('list_ui_workspaces');
  });

  it('returns nothing when the registry knows no workspace', async () => {
    invokeMock.mockResolvedValue(known([]));
    await expect(lastOpenedWorkspace()).resolves.toBeNull();
  });

  it('survives a failing registry read', async () => {
    invokeMock.mockRejectedValue(new Error('database locked'));
    await expect(lastOpenedWorkspace()).resolves.toBeNull();
  });
});
