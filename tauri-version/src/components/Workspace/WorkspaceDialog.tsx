import { t } from '../../i18n';
import { useCallback, useEffect, useState } from 'react';
import { Check, FolderOpen, Library, X } from 'lucide';
import { Icon } from '../Common/Icon';
import { Button } from '../Common/Button';
import { ConfirmDialog } from '../Common/ConfirmDialog';
import { Dialog, DialogFooter } from '../Common/Dialog';
import { EmptyState } from '../Common/EmptyState';
import { IconButton } from '../Common/IconButton';
import { ScrollArea } from '../Common/ScrollArea';
import {
  forgetWorkspace,
  listWorkspaces,
  pickWorkspaceFolder,
  type KnownWorkspace,
} from '../../modules/workspaces';
import { fileName, samePath } from '../../modules/paths';
import './WorkspaceDialog.css';

interface WorkspaceDialogProps {
  open: boolean;
  currentPath: string | null;
  onClose: () => void;
  onOpenWorkspace: (path: string) => Promise<boolean>;
}

export function WorkspaceDialog({ open, currentPath, onClose, onOpenWorkspace }: WorkspaceDialogProps) {
  const [workspaces, setWorkspaces] = useState<KnownWorkspace[] | null>(null);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [forgetTarget, setForgetTarget] = useState<KnownWorkspace | null>(null);

  const reload = useCallback(async () => {
    try {
      setWorkspaces(await listWorkspaces());
      setError(null);
    } catch (reason) {
      console.error('Failed to list workspaces', reason);
      setWorkspaces([]);
      setError(t('workspaces.listFailed', { reason: String(reason) }));
    }
  }, []);

  useEffect(() => {
    if (!open) {
      setError(null);
      setForgetTarget(null);
      setWorkspaces(null);
      return;
    }
    void reload();
  }, [open, reload]);

  const switchTo = useCallback(async (path: string) => {
    setPending(true);
    setError(null);
    try {
      if (await onOpenWorkspace(path)) onClose();
      else setError(t('workspaces.openFailed', { path }));
    } catch (reason) {
      console.error('Failed to open a workspace', reason);
      setError(t('workspaces.openFailed', { path }));
    } finally {
      setPending(false);
    }
  }, [onClose, onOpenWorkspace]);

  const pickFolder = useCallback(async () => {
    try {
      const path = await pickWorkspaceFolder();
      if (path) await switchTo(path);
    } catch (reason) {
      console.error('Failed to pick a workspace folder', reason);
    }
  }, [switchTo]);

  const confirmForget = useCallback(async () => {
    if (!forgetTarget) return;
    setPending(true);
    try {
      await forgetWorkspace(forgetTarget.id);
      await reload();
    } catch (reason) {
      console.error('Failed to forget a workspace', reason);
      setError(t('workspaces.forgetFailed'));
    } finally {
      setPending(false);
      setForgetTarget(null);
    }
  }, [forgetTarget, reload]);

  return (
    <>
      <Dialog
        open={open}
        title={t('workspaces.title')}
        closeLabel={t('workspaces.close')}
        className="q-workspace-dialog"
        onClose={onClose}
      >
        <ScrollArea className="q-workspace-dialog__list">
          {workspaces?.length === 0 ? (
            <EmptyState
              icon={Library}
              title={t('workspaces.empty')}
              description={t('workspaces.emptyHint')}
              compact
            />
          ) : workspaces?.map((workspace) => {
            const isCurrent = samePath(workspace.path, currentPath);
            return (
              <div
                key={workspace.id}
                className={`q-workspace-row${isCurrent ? ' q-workspace-row--current' : ''}`}
              >
                <button
                  type="button"
                  className="q-workspace-row__main"
                  disabled={pending || isCurrent}
                  title={workspace.path}
                  onClick={() => { void switchTo(workspace.path); }}
                >
                  <span className="q-workspace-row__icon" aria-hidden="true">
                    {isCurrent ? <Icon icon={Check} /> : null}
                  </span>
                  <span className="q-workspace-row__text">
                    <span className="q-workspace-row__name">{fileName(workspace.path)}</span>
                    <span className="q-workspace-row__path">{workspace.path}</span>
                  </span>
                </button>
                {isCurrent ? null : (
                  <IconButton
                    size="small"
                    className="q-workspace-row__forget"
                    label={t('workspaces.forget', { name: fileName(workspace.path) })}
                    disabled={pending}
                    onClick={() => setForgetTarget(workspace)}
                  >
                    <Icon icon={X} />
                  </IconButton>
                )}
              </div>
            );
          })}
        </ScrollArea>

        {error ? <p className="q-workspace-dialog__error" role="alert">{error}</p> : null}

        <DialogFooter>
          <Button disabled={pending} onClick={() => { void pickFolder(); }}>
            <Icon icon={FolderOpen} />
            {t('common.openFolder')}
          </Button>
        </DialogFooter>
      </Dialog>

      <ConfirmDialog
        open={forgetTarget !== null}
        title={t('workspaces.forgetTitle')}
        description={t('workspaces.forgetDescription', { name: forgetTarget ? fileName(forgetTarget.path) : '' })}
        confirmLabel={t('workspaces.forgetConfirm')}
        pendingLabel={t('workspaces.forgetPending')}
        pending={pending}
        onCancel={() => setForgetTarget(null)}
        onConfirm={() => { void confirmForget(); }}
      />
    </>
  );
}
