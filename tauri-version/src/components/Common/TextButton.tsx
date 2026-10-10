import type { ButtonHTMLAttributes } from 'react';
import './TextButton.css';

export function TextButton({
  className = '',
  type = 'button',
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement>) {
  return (
    <button
      {...props}
      type={type}
      className={`q-text-button ${className}`.trim()}
    />
  );
}
