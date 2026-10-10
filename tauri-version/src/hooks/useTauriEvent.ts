import { listen } from '@tauri-apps/api/event';
import { useEffect } from 'react';
import { useStableCallback } from './useStableCallback';

type Unsubscribe = () => void;

export function useTauriSubscription(
  subscribe: () => Promise<Unsubscribe>,
  name: string,
  enabled = true,
): void {
  const start = useStableCallback(subscribe);

  useEffect(() => {
    if (!enabled) return undefined;
    let disposed = false;
    let unsubscribe: Unsubscribe | null = null;
    void new Promise<Unsubscribe>((resolve) => resolve(start())).then((dispose) => {
      if (disposed) dispose();
      else unsubscribe = dispose;
    }).catch((error) => {
      if (!disposed) console.error(`Failed to subscribe to ${name}`, error);
    });
    return () => {
      disposed = true;
      unsubscribe?.();
    };
  }, [enabled, name, start]);
}

export function useTauriEvent<Payload>(
  event: string,
  handler: (payload: Payload) => void,
  enabled = true,
): void {
  const onEvent = useStableCallback(handler);
  useTauriSubscription(() => listen<Payload>(event, (message) => onEvent(message.payload)), event, enabled);
}
