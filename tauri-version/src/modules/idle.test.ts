import { afterEach, describe, expect, it, vi } from 'vitest';
import { whenIdle } from './idle';

afterEach(() => {
  vi.unstubAllGlobals();
  vi.useRealTimers();
});

describe('whenIdle', () => {
  it('waits for an idle moment instead of running straight away', () => {
    const idle: Array<() => void> = [];
    vi.stubGlobal('requestIdleCallback', (run: () => void) => {
      idle.push(run);
      return idle.length;
    });
    vi.stubGlobal('cancelIdleCallback', vi.fn());
    const work = vi.fn();

    whenIdle(work);

    expect(work).not.toHaveBeenCalled();
    idle.forEach((run) => run());
    expect(work).toHaveBeenCalledTimes(1);
  });

  it('asks the browser to give up waiting after a timeout', () => {
    const schedule = vi.fn(() => 1);
    vi.stubGlobal('requestIdleCallback', schedule);
    vi.stubGlobal('cancelIdleCallback', vi.fn());

    whenIdle(() => {}, 2_000);

    expect(schedule).toHaveBeenCalledWith(expect.any(Function), { timeout: 2_000 });
  });

  it('cancels the pending work', () => {
    const cancel = vi.fn();
    vi.stubGlobal('requestIdleCallback', () => 7);
    vi.stubGlobal('cancelIdleCallback', cancel);

    whenIdle(() => {})();

    expect(cancel).toHaveBeenCalledWith(7);
  });

  it('falls back to a timer where idle scheduling is missing', () => {
    vi.useFakeTimers();
    vi.stubGlobal('requestIdleCallback', undefined);
    const work = vi.fn();

    whenIdle(work);

    expect(work).not.toHaveBeenCalled();
    vi.runAllTimers();
    expect(work).toHaveBeenCalledTimes(1);
  });

  it('stops the fallback timer when cancelled', () => {
    vi.useFakeTimers();
    vi.stubGlobal('requestIdleCallback', undefined);
    const work = vi.fn();

    whenIdle(work)();
    vi.runAllTimers();

    expect(work).not.toHaveBeenCalled();
  });
});
