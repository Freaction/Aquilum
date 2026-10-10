import { roundCropPercent, type ImageCrop } from '../../../../modules/docs/imageEmbeds';
import { applyImageBox, type MediaElement } from './widgetDom';

const MIN_CROP_SIZE = 5;
const EDGES = ['nw', 'n', 'ne', 'e', 'se', 's', 'sw', 'w'] as const;

const FULL_CROP: ImageCrop = { left: 0, top: 0, right: 100, bottom: 100 };

export type CropSession = {
  apply: () => ImageCrop | null;
  cancel: () => void;
};

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, roundCropPercent(value)));
}

function isFullCrop(crop: ImageCrop): boolean {
  return crop.left === 0 && crop.top === 0 && crop.right === 100 && crop.bottom === 100;
}

function placeBox(box: HTMLElement, crop: ImageCrop): void {
  box.style.left = `${crop.left}%`;
  box.style.top = `${crop.top}%`;
  box.style.right = `${100 - crop.right}%`;
  box.style.bottom = `${100 - crop.bottom}%`;
}

export function startCropSession(
  clip: HTMLElement,
  image: MediaElement,
  initial: ImageCrop | null,
): CropSession {
  const crop: ImageCrop = { ...(initial ?? FULL_CROP) };
  applyImageBox(clip, image, null);

  const overlay = document.createElement('div');
  overlay.className = 'q-md-image-crop';

  const shade = document.createElement('div');
  shade.className = 'q-md-image-crop__shade';
  const dim = document.createElement('div');
  dim.className = 'q-md-image-crop__dim';
  shade.append(dim);

  const box = document.createElement('div');
  box.className = 'q-md-image-crop__box';

  for (const edge of EDGES) {
    const handle = document.createElement('div');
    handle.className = 'q-md-image-crop__handle';
    handle.dataset.edge = edge;
    box.append(handle);
  }

  const place = (): void => {
    placeBox(dim, crop);
    placeBox(box, crop);
  };
  place();

  overlay.append(shade, box);
  if (clip.parentElement) clip.after(overlay);
  else clip.append(overlay);

  const pointFrom = (event: PointerEvent) => {
    const rect = clip.getBoundingClientRect();
    return {
      x: ((event.clientX - rect.left) / rect.width) * 100,
      y: ((event.clientY - rect.top) / rect.height) * 100,
    };
  };

  const dragEdge = (event: PointerEvent, edge: string) => {
    const { x, y } = pointFrom(event);
    if (edge.includes('w')) crop.left = clamp(x, 0, crop.right - MIN_CROP_SIZE);
    if (edge.includes('e')) crop.right = clamp(x, crop.left + MIN_CROP_SIZE, 100);
    if (edge.includes('n')) crop.top = clamp(y, 0, crop.bottom - MIN_CROP_SIZE);
    if (edge.includes('s')) crop.bottom = clamp(y, crop.top + MIN_CROP_SIZE, 100);
    place();
  };

  const moveBox = (event: PointerEvent, start: { x: number; y: number }, from: ImageCrop) => {
    const { x, y } = pointFrom(event);
    const width = from.right - from.left;
    const height = from.bottom - from.top;
    crop.left = clamp(from.left + (x - start.x), 0, 100 - width);
    crop.top = clamp(from.top + (y - start.y), 0, 100 - height);
    crop.right = clamp(crop.left + width, 0, 100);
    crop.bottom = clamp(crop.top + height, 0, 100);
    place();
  };

  overlay.addEventListener('pointerdown', (event) => {
    if (event.button !== 0) return;
    event.preventDefault();
    event.stopPropagation();

    const target = event.target as HTMLElement;
    const edge = target.dataset.edge;
    const start = pointFrom(event);
    const from: ImageCrop = { ...crop };
    const onMove = edge
      ? (move: PointerEvent) => dragEdge(move, edge)
      : (move: PointerEvent) => moveBox(move, start, from);

    const onUp = () => {
      overlay.removeEventListener('pointermove', onMove);
      overlay.removeEventListener('pointerup', onUp);
      overlay.removeEventListener('pointercancel', onUp);
    };

    overlay.setPointerCapture(event.pointerId);
    overlay.addEventListener('pointermove', onMove);
    overlay.addEventListener('pointerup', onUp);
    overlay.addEventListener('pointercancel', onUp);
  });

  return {
    apply: () => {
      overlay.remove();
      return isFullCrop(crop) ? null : { ...crop };
    },
    cancel: () => {
      overlay.remove();
      applyImageBox(clip, image, initial);
    },
  };
}
