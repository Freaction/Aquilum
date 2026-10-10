import { useEffect, useId, useRef, type ReactNode } from 'react';
import { X } from 'lucide';
import { Icon } from './Icon';
import { IconButton } from './IconButton';
import { useStableCallback } from '../../hooks/useStableCallback';
import './Dialog.css';

function DialogHeader({ id, title, closeLabel, onClose }: {
  id: string;
  title: string;
  closeLabel?: string;
  onClose: () => void;
}) {
  return (
    <header className="q-dialog__header">
      <h2 id={id} className="q-dialog__title">{title}</h2>
      {closeLabel ? (
        <IconButton label={closeLabel} onClick={onClose}>
          <Icon icon={X} />
        </IconButton>
      ) : null}
    </header>
  );
}

export function DialogFooter({ align = 'end', children }: { align?: 'end' | 'center'; children: ReactNode }) {
  return (
    <footer className={`q-dialog__footer${align === 'center' ? ' q-dialog__footer--center' : ''}`}>
      {children}
    </footer>
  );
}

interface DialogProps {
  open: boolean;
  title: string;
  closeLabel?: string;
  headerless?: boolean;
  role?: 'dialog' | 'alertdialog';
  describedBy?: string;
  dismissible?: boolean;
  initialFocus?: () => void;
  className?: string;
  overlayClassName?: string;
  onClose: () => void;
  children: ReactNode;
}

export function Dialog({
  open,
  title,
  closeLabel,
  headerless = false,
  role = 'dialog',
  describedBy,
  dismissible = true,
  initialFocus,
  className = '',
  overlayClassName = '',
  onClose,
  children,
}: DialogProps) {
  const panelRef = useRef<HTMLElement>(null);
  const titleId = useId();
  const focusInitial = useStableCallback(() => {
    if (initialFocus) initialFocus();
    else if (!panelRef.current?.contains(document.activeElement)) panelRef.current?.focus();
  });

  useEffect(() => {
    if (!open) return;
    const frame = requestAnimationFrame(focusInitial);
    return () => cancelAnimationFrame(frame);
  }, [open, focusInitial]);

  if (!open) return null;

  const dismiss = () => {
    if (dismissible) onClose();
  };

  return (
    <div
      className={`q-dialog-overlay ${overlayClassName}`.trim()}
      role="presentation"
      onMouseDown={(event) => event.target === event.currentTarget && dismiss()}
    >
      <section
        ref={panelRef}
        tabIndex={-1}
        className={`q-dialog ${className}`.trim()}
        role={role}
        aria-modal="true"
        aria-labelledby={titleId}
        aria-describedby={describedBy}
        onKeyDown={(event) => {
          if (event.key !== 'Escape') return;
          event.preventDefault();
          dismiss();
        }}
      >
        {headerless ? (
          <h2 id={titleId} className="q-visually-hidden">{title}</h2>
        ) : (
          <DialogHeader id={titleId} title={title} closeLabel={closeLabel} onClose={dismiss} />
        )}
        {children}
      </section>
    </div>
  );
}
