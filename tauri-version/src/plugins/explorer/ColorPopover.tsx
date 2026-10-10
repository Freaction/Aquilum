import { Button } from '../../components/Common/Button';
import { Dialog, DialogFooter } from '../../components/Common/Dialog';
import { t } from '../../i18n';
import type { ColorToken } from '../vaultData';
import { usePickerFocus } from './usePickerFocus';
import './Pickers.css';

interface ColorPopoverProps {
  open: boolean;
  value?: ColorToken;
  onSelect: (value: ColorToken | null) => void;
  onClose: () => void;
  pending?: boolean;
  error?: string | null;
}

const COLORS: ColorToken[] = ['red', 'amber', 'green', 'teal', 'blue', 'purple', 'pink', 'gray'];

export function ColorPopover({ open, value, onSelect, onClose, pending = false, error }: ColorPopoverProps) {
  const bodyRef = usePickerFocus(open);
  return (
    <Dialog open={open} title={t('plugins.fileColors.pickerTitle')} closeLabel={t('plugins.fileColors.close')} onClose={onClose} className="q-color-picker">
      <div ref={bodyRef} className="q-picker__body" aria-busy={pending}>
        <div className="q-color-picker__palette">
          {COLORS.map(color => (
            <Button key={color} variant="ghost" className="q-color-picker__option" aria-pressed={value === color} disabled={pending} onClick={() => onSelect(color)}>
              <span className={`q-color-picker__swatch q-color-picker__swatch--${color}`} aria-hidden="true" />
              {t(`plugins.fileColors.colors.${color}`)}
            </Button>
          ))}
        </div>
        {error && <p className="q-picker__error" role="alert">{error}</p>}
      </div>
      <DialogFooter><Button variant="ghost" className="q-color-picker__remove" disabled={pending} onClick={() => onSelect(null)}>{t('plugins.fileColors.none')}</Button></DialogFooter>
    </Dialog>
  );
}
