import { getCurrentWindow } from '@tauri-apps/api/window';

// Окно создаётся скрытым (visible: false). В скрытом окне WKWebView на macOS
// не вызывает requestAnimationFrame, поэтому ждать только его нельзя: окно
// так и останется невидимым. Таймер показывает окно, если кадры не пошли.
const FRAME_FALLBACK_MS = 100;

let started = false;
let shown = false;

function showWindow(): void {
  if (shown) return;
  shown = true;

  void getCurrentWindow()
    .show()
    .catch((error) => console.error('Failed to reveal app window', error))
    .finally(() => document.documentElement.classList.add('q-window-revealed'));
}

export function revealAppWindow(): void {
  if (started) return;
  started = true;

  requestAnimationFrame(() => {
    requestAnimationFrame(showWindow);
  });
  setTimeout(showWindow, FRAME_FALLBACK_MS);
}
