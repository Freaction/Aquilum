import { forwardRef, type ReactNode } from 'react';
import { Minus, Plus } from 'lucide';
import { Icon } from './Icon';
import { CodeMirrorField, type CodeMirrorFieldRef } from './CodeMirrorField';
import { IconButton } from './IconButton';
import { t } from '../../i18n';
import './Input.css';

interface InputStepper {
  onStep: (direction: 1 | -1) => void;
  canDecrease: boolean;
  canIncrease: boolean;
}

interface InputFrameProps {
  counter?: ReactNode | false;
  disabled?: boolean;
  fullWidth?: boolean;
  startAdornment?: ReactNode;
  className?: string;
  children: ReactNode;
}

function InputFrame({ counter = false, disabled = false, fullWidth = false, startAdornment, className = '', children }: InputFrameProps) {
  return (
    <div className={`q-input${fullWidth ? ' q-input--full' : ''}${disabled ? ' q-input--disabled' : ''} ${className}`.trim()}>
      {startAdornment ? <span className="q-input__adornment">{startAdornment}</span> : null}
      {children}
      {counter !== false ? <span className="q-input__counter">{counter}</span> : null}
    </div>
  );
}

interface InputProps {
  value: string | number;
  onChange: (value: string) => void;
  ariaLabel: string;
  counter?: ReactNode | false;
  disabled?: boolean;
  fullWidth?: boolean;
  startAdornment?: ReactNode;
  placeholder?: string;
  className?: string;
  onBlur?: () => void;
  onKeyDown?: (event: KeyboardEvent) => boolean | void;
  stepper?: InputStepper;
}

export const Input = forwardRef<CodeMirrorFieldRef, InputProps>(function Input({
  value, onChange, ariaLabel, counter = false, disabled = false, fullWidth = false,
  startAdornment, placeholder, className = '', onBlur, onKeyDown, stepper,
}, ref) {
  const frameClass = stepper ? `q-input--stepper ${className}`.trim() : className;
  return (
    <InputFrame counter={counter} disabled={disabled} fullWidth={fullWidth} startAdornment={startAdornment} className={frameClass}>
      {stepper && (
        <IconButton label={t('common.decrease')} size="small" disabled={disabled || !stepper.canDecrease} onClick={() => stepper.onStep(-1)}>
          <Icon icon={Minus} />
        </IconButton>
      )}
      <CodeMirrorField
        ref={ref}
        className="q-input__field"
        value={String(value)}
        onChange={onChange}
        ariaLabel={ariaLabel}
        placeholder={placeholder}
        disabled={disabled}
        onBlur={onBlur}
        onKeyDown={(event) => onKeyDown?.(event)}
      />
      {stepper && (
        <IconButton label={t('common.increase')} size="small" disabled={disabled || !stepper.canIncrease} onClick={() => stepper.onStep(1)}>
          <Icon icon={Plus} />
        </IconButton>
      )}
    </InputFrame>
  );
});
