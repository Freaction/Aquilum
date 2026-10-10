type WheelHandler = (event: WheelEvent) => void;

interface BlockingWheelGesture {
  arm(): void;
  disarm(): void;
}

export function blockingWheelGesture(
  target: EventTarget,
  handler: WheelHandler,
): BlockingWheelGesture {
  let armed = false;
  const listener = handler as EventListener;

  return {
    arm() {
      if (armed) return;
      target.addEventListener('wheel', listener, { passive: false });
      armed = true;
    },
    disarm() {
      if (!armed) return;
      target.removeEventListener('wheel', listener);
      armed = false;
    },
  };
}
