import type { ComponentPropsWithRef, ReactNode } from 'react';

interface ButtonProps extends ComponentPropsWithRef<'button'> {
  variant?: 'primary' | 'ghost';
  size?: 'm' | 's' | 'xs';
  children: ReactNode;
}

export function Button({
  variant = 'primary',
  size = 'm',
  type = 'button',
  children,
  className = '',
  ...props
}: ButtonProps) {
  return (
    <button
      type={type}
      className={`q-button q-button--${size} q-button--${variant} ${className}`.trim()}
      {...props}
    >
      {children}
    </button>
  );
}
