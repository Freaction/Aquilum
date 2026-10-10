import { useEffect, useRef } from 'react';

export function usePickerFocus(open: boolean) {
  const bodyRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!open) return;
    const trigger = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    const panel = bodyRef.current?.closest<HTMLElement>('[role="dialog"]');
    if (!panel) return;
    const trap = (event: KeyboardEvent) => {
      if (event.key !== 'Tab') return;
      const controls = Array.from(panel.querySelectorAll<HTMLElement>('button:not([disabled]), [tabindex]:not([tabindex="-1"]), [contenteditable="true"]'))
        .filter(element => !element.hasAttribute('disabled') && element.getAttribute('aria-disabled') !== 'true');
      const first = controls[0]; const last = controls[controls.length - 1];
      if (!first || !last) { event.preventDefault(); panel.focus(); return; }
      if (document.activeElement === panel || (event.shiftKey ? document.activeElement === first : document.activeElement === last)) {
        event.preventDefault(); (event.shiftKey ? last : first).focus();
      }
    };
    panel.addEventListener('keydown', trap);
    return () => {
      panel.removeEventListener('keydown', trap);
      if (trigger?.isConnected) trigger.focus();
    };
  }, [open]);
  return bodyRef;
}
