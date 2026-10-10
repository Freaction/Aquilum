import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const show = vi.fn(() => Promise.resolve());
const addClass = vi.fn();

vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({ show }),
}));

async function freshReveal() {
  vi.resetModules();
  return (await import('./windowReveal')).revealAppWindow;
}

beforeEach(() => {
  vi.useFakeTimers();
  show.mockClear();
  addClass.mockClear();
  vi.stubGlobal('document', { documentElement: { classList: { add: addClass } } });
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.useRealTimers();
});

describe('revealAppWindow', () => {
  it('shows the window after two frames, once', async () => {
    const frames: Array<() => void> = [];
    vi.stubGlobal('requestAnimationFrame', (run: () => void) => frames.push(run));
    const revealAppWindow = await freshReveal();

    revealAppWindow();
    revealAppWindow();
    expect(show).not.toHaveBeenCalled();

    frames.shift()?.();
    expect(show).not.toHaveBeenCalled();
    frames.shift()?.();
    expect(show).toHaveBeenCalledTimes(1);

    await vi.runAllTimersAsync();
    expect(show).toHaveBeenCalledTimes(1);
    expect(addClass).toHaveBeenCalledWith('q-window-revealed');
  });

  it('shows the window even when frames never fire (hidden WKWebView on macOS)', async () => {
    vi.stubGlobal('requestAnimationFrame', () => 0);
    const revealAppWindow = await freshReveal();

    revealAppWindow();
    expect(show).not.toHaveBeenCalled();

    await vi.runAllTimersAsync();
    expect(show).toHaveBeenCalledTimes(1);
    expect(addClass).toHaveBeenCalledWith('q-window-revealed');
  });
});
