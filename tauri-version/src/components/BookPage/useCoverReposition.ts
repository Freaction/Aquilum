import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type PointerEvent as ReactPointerEvent,
} from 'react';
import {
  DEFAULT_COVER_POSITION,
  formatCoverPositionY,
  parseCoverPositionY,
} from '../../modules/docs/covers';
import { clamp } from '../../modules/math';

type Drag = {
  pointerId: number;
  startY: number;
  originY: number;
  overflow: number;
};

export function useCoverReposition(options: {
  enabled: boolean;
  imageUrl: string;
  position: string | undefined;
  onPositionChange?: (position: string) => void;
}) {
  const { enabled, imageUrl, position, onPositionChange } = options;
  const savedY = parseCoverPositionY(position);

  const [repositioning, setRepositioning] = useState(false);
  const [draftY, setDraftY] = useState(savedY);
  const [overflow, setOverflow] = useState(0);
  const dragRef = useRef<Drag | null>(null);
  const coverRef = useRef<HTMLDivElement | null>(null);
  const imageRef = useRef<HTMLImageElement | null>(null);
  const draftYRef = useRef(draftY);
  draftYRef.current = draftY;

  const measureOverflow = useCallback(() => {
    const cover = coverRef.current;
    const image = imageRef.current;
    if (!cover || !image) return;
    const { naturalWidth, naturalHeight } = image;
    if (!naturalWidth || !naturalHeight) return;
    const scale = Math.max(cover.clientWidth / naturalWidth, cover.clientHeight / naturalHeight);
    setOverflow(Math.max(0, naturalHeight * scale - cover.clientHeight));
  }, []);

  useLayoutEffect(() => {
    if (!enabled) return;
    measureOverflow();
  }, [enabled, imageUrl, measureOverflow, repositioning]);

  useEffect(() => {
    if (!enabled) return;
    const cover = coverRef.current;
    const image = imageRef.current;
    if (!cover || !image) return;
    const observer = new ResizeObserver(() => measureOverflow());
    observer.observe(cover);
    observer.observe(image);
    return () => observer.disconnect();
  }, [enabled, measureOverflow]);

  useEffect(() => {
    if (!repositioning) setDraftY(savedY);
  }, [savedY, repositioning]);

  useEffect(() => {
    if (enabled) return;
    setRepositioning(false);
  }, [enabled]);

  const commitReposition = useCallback(() => {
    const next = formatCoverPositionY(draftYRef.current);
    setRepositioning(false);
    const current = position || DEFAULT_COVER_POSITION;
    if (next === current) return;
    onPositionChange?.(next === DEFAULT_COVER_POSITION ? '' : next);
  }, [onPositionChange, position]);

  const cancelReposition = useCallback(() => {
    setDraftY(savedY);
    setRepositioning(false);
  }, [savedY]);

  useEffect(() => {
    if (!repositioning) return;

    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        event.preventDefault();
        cancelReposition();
        return;
      }
      if (event.key === 'Enter') {
        event.preventDefault();
        commitReposition();
      }
    };

    const onPointerDownOutside = (event: PointerEvent) => {
      const cover = coverRef.current;
      if (!cover) return;
      if (event.target instanceof Node && cover.contains(event.target)) return;
      commitReposition();
    };

    window.addEventListener('keydown', onKey);
    window.addEventListener('pointerdown', onPointerDownOutside, true);
    return () => {
      window.removeEventListener('keydown', onKey);
      window.removeEventListener('pointerdown', onPointerDownOutside, true);
    };
  }, [cancelReposition, commitReposition, repositioning]);

  const toggleReposition = useCallback(() => {
    if (repositioning) {
      commitReposition();
      return;
    }
    setDraftY(savedY);
    setRepositioning(true);
  }, [commitReposition, repositioning, savedY]);

  const onPointerDown = useCallback((event: ReactPointerEvent<HTMLDivElement>) => {
    if (!repositioning || event.button !== 0) return;
    event.preventDefault();
    event.currentTarget.setPointerCapture(event.pointerId);
    dragRef.current = {
      pointerId: event.pointerId,
      startY: event.clientY,
      originY: draftY,
      overflow,
    };
  }, [draftY, overflow, repositioning]);

  const onPointerMove = useCallback((event: ReactPointerEvent<HTMLDivElement>) => {
    const drag = dragRef.current;
    if (!drag || drag.pointerId !== event.pointerId) return;
    if (drag.overflow <= 0) return;
    const dy = event.clientY - drag.startY;
    const nextY = drag.originY - (dy / drag.overflow) * 100;
    setDraftY(clamp(nextY, 0, 100));
  }, []);

  const onPointerUp = useCallback((event: ReactPointerEvent<HTMLDivElement>) => {
    const drag = dragRef.current;
    if (!drag || drag.pointerId !== event.pointerId) return;
    dragRef.current = null;
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
  }, []);

  return {
    coverRef,
    imageRef,
    repositioning,
    y: repositioning ? draftY : savedY,
    measureOverflow,
    toggleReposition,
    onPointerDown,
    onPointerMove,
    onPointerUp,
  };
}
