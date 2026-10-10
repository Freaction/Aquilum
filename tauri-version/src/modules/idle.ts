const DEFAULT_TIMEOUT_MS = 1_500;

export function whenIdle(run: () => void, timeoutMs = DEFAULT_TIMEOUT_MS): () => void {
  const schedule = typeof requestIdleCallback === 'function' ? requestIdleCallback : null;
  if (schedule) {
    const handle = schedule(() => run(), { timeout: timeoutMs });
    return () => cancelIdleCallback(handle);
  }
  const handle = setTimeout(run, 0);
  return () => clearTimeout(handle);
}
