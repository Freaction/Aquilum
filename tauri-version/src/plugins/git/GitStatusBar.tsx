import { useCallback, useEffect, useRef, useState } from 'react';
import { GitBranch, LoaderCircle, RefreshCw } from 'lucide';
import { useSettingsStore, type PluginSettings } from '../../modules/settings';
import { useTauriEvent } from '../../hooks/useTauriEvent';
import { t } from '../../i18n';
import { Icon } from '../../components/Common/Icon';
import { Button } from '../../components/Common/Button';
import { gitStatus, gitPull, gitSync, type GitStatus, type GitError } from './api';
import './GitStatusBar.css';

export function GitStatusBar({ workspacePath }: { workspacePath: string | null }) {
  const { config } = useSettingsStore();
  const settings = config?.plugins.gitSync;
  if (!workspacePath || !settings?.enabled) return null;
  return <VaultGitStatus key={workspacePath} workspacePath={workspacePath} settings={settings} />;
}

function VaultGitStatus({ workspacePath, settings }: { workspacePath: string; settings: PluginSettings['gitSync'] }) {
  const [status, setStatus] = useState<GitStatus | null>(null);
  const [error, setError] = useState<GitError | null>(null);
  const [busy, setBusy] = useState(false);
  const locked = useRef(false);
  const pendingRefresh = useRef(false);
  const mounted = useRef(false);
  const refreshTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const run = useCallback(async (action: 'status' | 'pull' | 'sync', onlyChanged = false): Promise<void> => {
    if (locked.current) {
      if (action === 'status') pendingRefresh.current = true;
      return;
    }
    locked.current = true;
    setBusy(true);
    try {
      let result: GitStatus;
      if (onlyChanged) {
        result = await gitStatus(workspacePath);
        if (mounted.current && result.changed > 0 && result.conflicted.length === 0) result = await gitSync(workspacePath);
      } else {
        result = await (action === 'sync' ? gitSync : action === 'pull' ? gitPull : gitStatus)(workspacePath);
      }
      if (mounted.current) {
        setStatus(result);
        setError(previous => action !== 'status' || onlyChanged
          || previous?.code === 'not_a_repository' || previous?.code === 'git_not_found' ? null : previous);
      }
    } catch (failure) {
      if (mounted.current) {
        const failureDetails = failure as Partial<GitError> | null;
        setError(failureDetails?.code && failureDetails.details
          ? failureDetails as GitError
          : { code: 'failed', details: { message: String(failure) } });
      }
    } finally {
      locked.current = false;
      if (mounted.current) {
        setBusy(false);
        if (pendingRefresh.current) {
          pendingRefresh.current = false;
          void run('status');
        }
      }
    }
  }, [workspacePath]);

  useEffect(() => {
    mounted.current = true;
    void run(settings.pullOnOpen ? 'pull' : 'status');
    return () => {
      mounted.current = false;
      if (refreshTimer.current !== null) clearTimeout(refreshTimer.current);
    };
    // Pull выполняется один раз при открытии хранилища.
  }, [run]);

  useEffect(() => {
    if (settings.autoBackupMinutes <= 0) return;
    const timer = setInterval(() => { void run('sync', true); }, Math.min(settings.autoBackupMinutes * 60_000, 2_147_483_647));
    return () => clearInterval(timer);
  }, [settings.autoBackupMinutes, run]);

  useTauriEvent('workspace-changed', () => {
    if (refreshTimer.current !== null) clearTimeout(refreshTimer.current);
    refreshTimer.current = setTimeout(() => { void run('status'); }, 150);
  });

  const unavailable = error?.code === 'not_a_repository' || error?.code === 'git_not_found';
  return <div className="q-git-status" aria-busy={busy}>
    <div className="q-git-status__row">
      <Icon icon={GitBranch} size={16} />
      <span className="q-git-status__branch" title={status?.branch}>{unavailable ? t('plugins.gitSync.unavailable') : status?.branch ?? t('plugins.gitSync.loading')}</span>
      {status && <span className="q-git-status__counts" aria-label={t('plugins.gitSync.status', { ahead: status.ahead, behind: status.behind, changed: status.changed })}>
        ↑{status.ahead} ↓{status.behind} · {status.changed}
      </span>}
      <Button variant="ghost" size="xs" disabled={busy || unavailable} aria-label={t('plugins.gitSync.sync')} title={t('plugins.gitSync.sync')} onClick={() => { void run('sync'); }}>
        <Icon icon={busy ? LoaderCircle : RefreshCw} size={16} className={busy ? 'q-git-status__spinner' : undefined} />
      </Button>
    </div>
    {error && <div className="q-git-status__error" role="alert">
      <span>{t(`plugins.gitSync.errors.${error.code}`)}{error.details.message && `: ${error.details.message}`}</span>
      {!!error.details.files?.length && <ul>{error.details.files.map(file => <li key={file}>{file}</li>)}</ul>}
    </div>}
  </div>;
}
