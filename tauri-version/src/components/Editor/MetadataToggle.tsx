import { t } from '../../i18n';
import { ChevronDown, ChevronRight } from 'lucide';
import { Icon } from '../Common/Icon';

type MetadataToggleProps = {
  expanded: boolean;
  onToggle: () => void;
};

export function MetadataToggle({ expanded, onToggle }: MetadataToggleProps) {
  return (
    <button
      type="button"
      className="q-metadata-toggle"
      aria-expanded={expanded}
      onMouseDown={(event) => {
        event.preventDefault();
        onToggle();
      }}
    >
      <span className="q-metadata-toggle__chevron" aria-hidden="true">
        {expanded
          ? <Icon icon={ChevronDown} size={20} strokeWidth={1.5} />
          : <Icon icon={ChevronRight} size={20} strokeWidth={1.5} />}
      </span>
      <span className="q-metadata-toggle__label">{t('editor.metadata')}</span>
    </button>
  );
}
