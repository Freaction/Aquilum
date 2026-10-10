import type { EditorView } from '@codemirror/view';
import { openImageViewer } from '../../../Common/imageViewer';
import {
  formatImageEmbed,
  nextImageAlign,
  parseImageEmbed,
  type ImageAlign,
  type ImageCrop,
  type ImageEmbed,
} from '../../../../modules/docs/imageEmbeds';
import { startCropSession, type CropSession } from './crop';
import {
  cachedImageSize,
  createImageChrome,
  imageAreaWidth,
  imageBoxAspect,
  imageUrlOf,
  type MediaElement,
} from './widgetDom';
import { pickImage, showImageSource } from './focus';

const MIN_IMAGE_WIDTH = 96;
const MIN_IMAGE_HEIGHT = 48;

function embedAt(view: EditorView, root: HTMLElement) {
  const line = view.state.doc.lineAt(view.posAtDOM(root));
  const embed = parseImageEmbed(line.text);
  return embed ? { line, embed } : null;
}

function rewriteImageEmbed(
  view: EditorView,
  root: HTMLElement,
  change: (embed: ImageEmbed) => ImageEmbed,
): boolean {
  const current = embedAt(view, root);
  if (!current) return false;

  view.dispatch({
    changes: {
      from: current.line.from,
      to: current.line.to,
      insert: formatImageEmbed(change(current.embed)),
    },
    userEvent: 'input',
  });
  return true;
}

function imageLineFrom(view: EditorView, root: HTMLElement): number {
  return view.state.doc.lineAt(view.posAtDOM(root)).from;
}

function imageUrl(root: HTMLElement): string {
  return root.querySelector<MediaElement>('.q-md-image-img')?.src ?? '';
}

function isVideo(root: HTMLElement): boolean {
  return root.dataset.kind === 'video';
}

function toolbarWidthFloor(root: HTMLElement): number {
  const toolbar = root.querySelector<HTMLElement>('.q-md-image-toolbar');
  if (!toolbar) return 0;
  const inset = parseFloat(getComputedStyle(toolbar).right || '0');
  return Math.ceil(toolbar.getBoundingClientRect().width + inset * 2);
}

function minImageWidth(root: HTMLElement, crop: ImageCrop | null): number {
  const image = root.querySelector<MediaElement>('.q-md-image-img');
  const aspect = image ? imageBoxAspect(cachedImageSize(imageUrlOf(image)), crop) : null;
  const floor = Math.max(MIN_IMAGE_WIDTH, toolbarWidthFloor(root));
  if (!aspect) return floor;
  return Math.max(floor, Math.round(MIN_IMAGE_HEIGHT * aspect));
}

function bindResize(
  view: EditorView,
  root: HTMLElement,
  wrap: HTMLElement,
  grip: HTMLElement,
  onIdle: () => void,
): void {
  grip.addEventListener('pointerdown', (event) => {
    if (event.button !== 0) return;
    event.preventDefault();
    event.stopPropagation();

    const step = root.dataset.align === 'center' ? 2 : 1;
    const maxWidth = imageAreaWidth(root);
    const minWidth = minImageWidth(root, embedAt(view, root)?.embed.crop ?? null);
    const startX = event.clientX;
    const startWidth = wrap.getBoundingClientRect().width;
    let width = Math.round(startWidth);

    root.dataset.resizing = 'true';
    grip.setPointerCapture(event.pointerId);

    const onMove = (move: PointerEvent) => {
      const delta = (move.clientX - startX) * step;
      width = Math.round(Math.min(maxWidth, Math.max(minWidth, startWidth + delta)));
      wrap.style.width = `${width}px`;
    };

    const onUp = () => {
      grip.removeEventListener('pointermove', onMove);
      grip.removeEventListener('pointerup', onUp);
      grip.removeEventListener('pointercancel', onUp);
      delete root.dataset.resizing;
      rewriteImageEmbed(view, root, (embed) => ({ ...embed, width }));
      onIdle();
    };

    grip.addEventListener('pointermove', onMove);
    grip.addEventListener('pointerup', onUp);
    grip.addEventListener('pointercancel', onUp);
  });
}

export function bindImageEmbedEvents(view: EditorView, root: HTMLElement): void {
  const wrap = root.querySelector<HTMLElement>('.q-md-image-wrap');
  const clip = root.querySelector<HTMLElement>('.q-md-image-clip');
  const image = root.querySelector<MediaElement>('.q-md-image-img');
  if (!wrap || !clip || !image) return;

  let chrome: HTMLElement[] = [];
  let cropSession: CropSession | null = null;
  let hovered = false;

  const cropButton = () => root.querySelector<HTMLElement>('[data-image-action="crop"]');

  const stopCropKeys = () => document.removeEventListener('keydown', onCropKeyDown, true);

  const finishCrop = (keep: boolean) => {
    const session = cropSession;
    if (!session) return;
    cropSession = null;
    delete root.dataset.cropping;
    cropButton()?.classList.remove('active');
    stopCropKeys();

    if (keep) {
      const crop = session.apply();
      rewriteImageEmbed(view, root, (embed) => ({ ...embed, crop }));
    } else {
      session.cancel();
    }
    hideChrome();
  };

  function onCropKeyDown(event: KeyboardEvent) {
    if (!root.isConnected) {
      cropSession = null;
      stopCropKeys();
      return;
    }
    if (event.key !== 'Escape' && event.key !== 'Enter') return;
    event.preventDefault();
    event.stopPropagation();
    finishCrop(event.key === 'Enter');
  }

  const startCrop = () => {
    cropSession = startCropSession(clip, image, embedAt(view, root)?.embed.crop ?? null);
    root.dataset.cropping = 'true';
    cropButton()?.classList.add('active');
    document.addEventListener('keydown', onCropKeyDown, true);
  };

  const bindToolbar = (toolbar: HTMLElement) => {
    toolbar.addEventListener('pointerdown', (event) => {
      event.preventDefault();
      event.stopPropagation();
    });

    toolbar.addEventListener('click', (event) => {
      const button = (event.target as HTMLElement | null)?.closest<HTMLElement>('[data-image-action]');
      if (!button) return;
      event.preventDefault();
      event.stopPropagation();

      switch (button.dataset.imageAction) {
        case 'zoom':
          openImageViewer(imageUrl(root));
          break;
        case 'align':
          rewriteImageEmbed(view, root, (embed) => ({ ...embed, align: nextImageAlign(embed.align) }));
          break;
        case 'crop':
          if (cropSession) finishCrop(true);
          else startCrop();
          break;
        case 'code':
          finishCrop(false);
          showImageSource(view, imageLineFrom(view, root));
          break;
        default:
          break;
      }
    });
  };

  const showChrome = () => {
    if (chrome.length) return;
    const align = (root.dataset.align ?? 'center') as ImageAlign;
    chrome = createImageChrome(align, isVideo(root) ? 'video' : 'image');
    for (const element of chrome) {
      if (element.classList.contains('q-md-image-grip')) bindResize(view, root, wrap, element, hideChrome);
      else bindToolbar(element);
    }
    wrap.append(...chrome);
    if (cropSession) cropButton()?.classList.add('active');
  };

  function hideChrome(): void {
    if (hovered || root.dataset.resizing || cropSession) return;
    for (const element of chrome) element.remove();
    chrome = [];
  }

  wrap.addEventListener('pointerenter', () => {
    hovered = true;
    showChrome();
  });
  wrap.addEventListener('pointerleave', () => {
    hovered = false;
    hideChrome();
  });

  root.addEventListener('mousedown', (event) => {
    if (event.button !== 0 || cropSession) return;
    if (!(event.target as HTMLElement | null)?.closest('.q-md-image-clip')) return;
    pickImage(root);
    if (!isVideo(root)) event.preventDefault();
    view.focus();
  });

  root.addEventListener('dblclick', (event) => {
    if (cropSession || isVideo(root)) return;
    event.preventDefault();
    openImageViewer(imageUrl(root));
  });
}
