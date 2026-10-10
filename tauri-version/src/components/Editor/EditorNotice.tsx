import { Button } from '../Common/Button';
import './EditorNotice.css';

interface EditorNoticeProps {
  message: string;
  actionLabel: string;
  onAction: () => void;
}

export function EditorNotice({ message, actionLabel, onAction }: EditorNoticeProps) {
  return (
    <div className="q-editor-notice q-selectable" role="alert">
      <span className="q-editor-notice__text">{message}</span>
      <Button variant="ghost" size="xs" onClick={onAction}>{actionLabel}</Button>
    </div>
  );
}
