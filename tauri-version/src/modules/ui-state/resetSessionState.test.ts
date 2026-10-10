import { afterEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { resetSessionState } from './gateway';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn().mockResolvedValue(undefined) }));

describe('resetSessionState', () => {
  afterEach(() => vi.unstubAllGlobals());

  it('resets the tab state, keeps the current knowledge base and restarts', async () => {
    const reload = vi.fn();
    vi.stubGlobal('window', { location: { reload } });

    await resetSessionState('C:/vault');

    expect(vi.mocked(invoke).mock.calls.map(([command]) => command)).toEqual(['reset_ui_state', 'resolve_ui_workspace']);
    expect(invoke).toHaveBeenLastCalledWith('resolve_ui_workspace', { path: 'C:/vault', nowMs: expect.any(Number) });
    expect(reload).toHaveBeenCalled();
  });
});
