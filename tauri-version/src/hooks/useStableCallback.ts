import { useCallback, useInsertionEffect, useRef } from 'react';

export function useStableCallback<Args extends unknown[], Result>(
  callback: (...args: Args) => Result,
): (...args: Args) => Result {
  const latest = useRef(callback);

  useInsertionEffect(() => {
    latest.current = callback;
  }, [callback]);

  return useCallback((...args: Args) => latest.current(...args), []);
}
