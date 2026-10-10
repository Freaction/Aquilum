import { InlineRenameField } from '../Common/InlineRenameField';
import { t } from '../../i18n';

interface FileTreeRenameProps {
  name: string;
  className: string;
  active: boolean;
  onCommit: (nextName: string) => void;
  onCancel: () => void;
}

export function FileTreeRename({
  name,
  className,
  active,
  onCommit,
  onCancel,
}: FileTreeRenameProps) {
  return (
    <div className={className} data-file-active={active || undefined}>
      <span className="q-file-icon" aria-hidden="true" />
      <InlineRenameField
        name={name}
        ariaLabel={t('fileTree.renameAria')}
        className="q-file-rename-field"
        onCommit={onCommit}
        onCancel={onCancel}
      />
    </div>
  );
}
