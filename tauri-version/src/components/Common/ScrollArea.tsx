import { forwardRef, type HTMLAttributes } from 'react';
import './ScrollArea.css';

export const ScrollArea = forwardRef<HTMLDivElement, HTMLAttributes<HTMLDivElement>>(
  function ScrollArea({ className = '', ...props }, ref) {
    return <div ref={ref} className={`q-scroll-area ${className}`.trim()} {...props} />;
  },
);
