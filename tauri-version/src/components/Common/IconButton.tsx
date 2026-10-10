import type { ButtonHTMLAttributes, ReactNode } from 'react';
import './IconButton.css';

interface IconButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
    children: ReactNode;
    label?: string;
    size?: 'small' | 'medium';
    variant?: 'default' | 'white';
}

export function IconButton({
    children,
    className = '',
    label,
    size = 'medium',
    title,
    type = 'button',
    variant = 'default',
    ...props
}: IconButtonProps) {
    const variantClass = variant === 'default' ? '' : ` q-icon-button--${variant}`;
    const classes = `q-icon-button q-icon-button--${size}${variantClass} ${className}`.trim();

    return (
        <button
            {...props}
            type={type}
            className={classes}
            {...(label ? { 'aria-label': label } : {})}
            {...(title || label ? { title: title ?? label } : {})}
        >
            {children}
        </button>
    );
}
