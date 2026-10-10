import { useId, useRef } from 'react';
import { Button } from './Button';
import { Dialog, DialogFooter } from './Dialog';
import './ConfirmDialog.css';
import { t } from '../../i18n';

interface ConfirmDialogProps {
  open: boolean;
  title: string;
  description: string;
  confirmLabel: string;
  pendingLabel?: string;
  pending?: boolean;
  onCancel: () => void;
  onConfirm: () => void;
}

export function ConfirmDialog({ open, title, description, confirmLabel, pendingLabel = t('confirm.deleting'), pending = false, onCancel, onConfirm }: ConfirmDialogProps) {
  const confirmRef = useRef<HTMLButtonElement>(null);
  const descriptionId = useId();

  return (
    <Dialog
      open={open}
      title={title}
      role="alertdialog"
      describedBy={descriptionId}
      dismissible={!pending}
      initialFocus={() => confirmRef.current?.focus()}
      className="q-confirm-dialog"
      onClose={onCancel}
    >
      <div className="q-confirm-dialog__content">
        <p id={descriptionId}>{description}</p>
      </div>
      <DialogFooter>
        <Button variant="ghost" disabled={pending} onClick={onCancel}>{t('common.cancel')}</Button>
        <Button ref={confirmRef} disabled={pending} onClick={onConfirm}>
          {pending ? pendingLabel : confirmLabel}
        </Button>
      </DialogFooter>
    </Dialog>
  );
}
