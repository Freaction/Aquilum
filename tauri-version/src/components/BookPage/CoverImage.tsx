import { forwardRef, type CSSProperties } from 'react';

type CoverImageProps = {
  src: string;
  className?: string;
  style?: CSSProperties;
  onLoad?: () => void;
};

export const CoverImage = forwardRef<HTMLImageElement, CoverImageProps>(function CoverImage(
  { src, className, style, onLoad },
  ref,
) {
  const normalized = src.trim();
  if (!normalized) return null;

  return (
    <img
      ref={ref}
      src={normalized}
      alt=""
      className={className}
      style={style}
      draggable={false}
      onLoad={onLoad}
    />
  );
});
