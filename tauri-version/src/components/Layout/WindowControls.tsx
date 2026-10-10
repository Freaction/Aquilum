import { useEffect, useState } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { isMacOs } from '../../modules/platform';
import './WindowControls.css';
import { t } from '../../i18n';

const appWindow = getCurrentWindow();
const isMac = isMacOs();

export function WindowControls() {
  if (isMac) return null;
  return <TrailingWindowControls />;
}

function TrailingWindowControls() {
  const [isMaximized, setIsMaximized] = useState(false);

  useEffect(() => {
    const syncMaximized = async () => setIsMaximized(await appWindow.isMaximized());
    void syncMaximized();
    const unlisten = appWindow.onResized(() => void syncMaximized());
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const handleToggleMaximize = async () => {
    if (await appWindow.isMaximized()) appWindow.unmaximize();
    else appWindow.maximize();
  };

  return (
    <div className="q-window-controls">
      <button type="button" onClick={() => appWindow.minimize()} title={t('window.minimize')} aria-label={t('window.minimize')} className="q-window-btn q-window-btn--min">
        <svg aria-hidden="true" width="12" height="12" viewBox="0 0 12 12">
          <rect fill="currentColor" width="10" height="1" x="1" y="6" />
        </svg>
      </button>
      <button type="button" onClick={handleToggleMaximize} title={t('window.maximize')} aria-label={t('window.maximize')} className="q-window-btn q-window-btn--max">
        {isMaximized ? (
          <svg aria-hidden="true" width="12" height="12" viewBox="0 0 12 12" fill="none">
            <path d="M1.5 3.5H8.5V10.5H1.5V3.5Z" stroke="currentColor" />
            <path d="M4 2H10V8H9V9H11V1H3V3H4V2Z" fill="currentColor" />
          </svg>
        ) : (
          <svg aria-hidden="true" width="12" height="12" viewBox="0 0 12 12" fill="none">
            <rect width="9" height="9" x="1.5" y="1.5" stroke="currentColor" />
          </svg>
        )}
      </button>
      <button type="button" onClick={() => appWindow.close()} title={t('window.close')} aria-label={t('window.close')} className="q-window-btn q-window-btn--close">
        <svg aria-hidden="true" width="12" height="12" viewBox="0 0 12 12">
          <path fill="currentColor" fillRule="evenodd" d="M10.052 10.968 1.03 1.93l.849-.848 9.023 9.037-.849.848Z" />
          <path fill="currentColor" fillRule="evenodd" d="M1.023 10.112 10.06 1.09l.848.85-9.037 9.023-.848-.85Z" />
        </svg>
      </button>
    </div>
  );
}
