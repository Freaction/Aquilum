import {
  AlignCenter,
  AlignLeft,
  AlignRight,
  CodeXml,
  Crop,
  MoveDiagonal2,
  ZoomIn,
  type IconNode,
} from 'lucide';
import type { EditorView } from '@codemirror/view';
import { createIconElement } from '../../../Common/iconElement';
import type { ImageAlign, ImageCrop, MediaKind } from '../../../../modules/docs/imageEmbeds';

export type MediaElement = HTMLImageElement | HTMLVideoElement;

type ImageEmbedAction = 'zoom' | 'align' | 'crop' | 'code';

type ImageEmbedViewData = {
  url: string;
  width: number | null;
  align: ImageAlign;
  crop: ImageCrop | null;
  kind: MediaKind;
};

const ALIGN_ICONS: Record<ImageAlign, IconNode> = {
  left: AlignLeft,
  center: AlignCenter,
  right: AlignRight,
};

const ALIGN_TITLES: Record<ImageAlign, string> = {
  left: 'Выравнивание слева',
  center: 'Выравнивание по центру',
  right: 'Выравнивание справа',
};

function actionButton(action: ImageEmbedAction, icon: IconNode, title: string): HTMLElement {
  const button = document.createElement('button');
  button.type = 'button';
  button.className = 'q-icon-button q-icon-button--small';
  button.dataset.imageAction = action;
  button.title = title;
  button.replaceChildren(createIconElement(icon));
  return button;
}

function divider(): HTMLElement {
  const element = document.createElement('div');
  element.className = 'q-floating-actions__divider';
  return element;
}

const mediaByUrl = new Map<string, MediaElement>();

const MAX_POOLED_MEDIA = 8;

function createVideoElement(url: string): HTMLVideoElement {
  const video = document.createElement('video');
  video.className = 'q-md-image-img';
  video.controls = true;
  video.preload = 'metadata';
  video.playsInline = true;
  if (url) video.src = url;
  return video;
}

function createImageElement(url: string): HTMLImageElement {
  const image = document.createElement('img');
  image.className = 'q-md-image-img';
  image.alt = '';
  image.draggable = false;
  if (url) image.src = url;
  return image;
}

function acquireMediaElement(url: string, kind: MediaKind): MediaElement {
  const pooled = url ? mediaByUrl.get(url) : undefined;
  if (pooled && !pooled.isConnected) return pooled;

  const element = kind === 'video' ? createVideoElement(url) : createImageElement(url);
  if (url && !pooled) {
    mediaByUrl.set(url, element);
    if (mediaByUrl.size > MAX_POOLED_MEDIA) {
      const oldest = mediaByUrl.keys().next().value;
      if (oldest !== undefined) mediaByUrl.delete(oldest);
    }
  }
  return element;
}

type ImageNaturalSize = { width: number; height: number };

const naturalSizeByUrl = new Map<string, ImageNaturalSize>();

export function cachedImageSize(url: string): ImageNaturalSize | undefined {
  return naturalSizeByUrl.get(url);
}

export function imageUrlOf(image: MediaElement): string {
  return image.getAttribute('src') ?? '';
}

function mediaNaturalSize(media: MediaElement): ImageNaturalSize | undefined {
  const width = media instanceof HTMLVideoElement ? media.videoWidth : media.naturalWidth;
  const height = media instanceof HTMLVideoElement ? media.videoHeight : media.naturalHeight;
  return width && height ? { width, height } : undefined;
}

export function rememberImageSize(media: MediaElement): ImageNaturalSize | undefined {
  const size = mediaNaturalSize(media);
  if (!size) return undefined;
  naturalSizeByUrl.set(imageUrlOf(media), size);
  return size;
}

function contentBoxWidth(element: HTMLElement): number {
  const style = getComputedStyle(element);
  return element.clientWidth
    - parseFloat(style.paddingLeft || '0')
    - parseFloat(style.paddingRight || '0');
}

let measuredImageArea = 0;

export function imageAreaWidth(root: HTMLElement): number {
  const width = contentBoxWidth(root);
  if (width > 0) measuredImageArea = Math.round(width);
  return width;
}

export function lastImageArea(): number {
  return measuredImageArea;
}

export function editorColumnWidth(view: EditorView): number | null {
  const width = Math.round(contentBoxWidth(view.contentDOM));
  return width > 0 ? width : null;
}

const VIDEO_FALLBACK_ASPECT = 16 / 9;

export function imageBoxAspect(
  size: ImageNaturalSize | undefined,
  crop: ImageCrop | null,
  kind: MediaKind = 'image',
): number | null {
  if (!size) return kind === 'video' ? VIDEO_FALLBACK_ASPECT : null;
  if (!crop) return size.width / size.height;
  return (size.width * (crop.right - crop.left)) / (size.height * (crop.bottom - crop.top));
}

const SUPPORTS_VIEW_BOX = typeof CSS !== 'undefined'
  && typeof CSS.supports === 'function'
  && CSS.supports('object-view-box', 'inset(0% 0% 0% 0%)');

function clearBox(media: MediaElement): void {
  media.style.position = '';
  media.style.width = '';
  media.style.height = '';
  media.style.left = '';
  media.style.top = '';
  media.style.removeProperty('object-view-box');
}

function cropThroughContentBox(media: MediaElement, crop: ImageCrop): void {
  media.style.position = 'absolute';
  media.style.left = '0';
  media.style.top = '0';
  media.style.width = '100%';
  media.style.height = '100%';
  media.style.setProperty(
    'object-view-box',
    `inset(${crop.top}% ${100 - crop.right}% ${100 - crop.bottom}% ${crop.left}%)`,
  );
}

export function applyImageBox(
  clip: HTMLElement,
  image: MediaElement,
  crop: ImageCrop | null,
  size = cachedImageSize(imageUrlOf(image)),
): void {
  const kind: MediaKind = image instanceof HTMLVideoElement ? 'video' : 'image';
  const aspect = imageBoxAspect(size, crop, kind);
  clip.style.aspectRatio = aspect ? String(aspect) : '';
  clearBox(image);

  if (!crop) return;

  if (image instanceof HTMLVideoElement && SUPPORTS_VIEW_BOX) {
    cropThroughContentBox(image, crop);
    return;
  }

  const scaleX = 100 / (crop.right - crop.left);
  const scaleY = 100 / (crop.bottom - crop.top);

  image.style.position = 'absolute';
  image.style.width = `${scaleX * 100}%`;
  image.style.height = `${scaleY * 100}%`;
  image.style.left = `${-crop.left * scaleX}%`;
  image.style.top = `${-crop.top * scaleY}%`;
}

export function renderImageEmbedDom(data: ImageEmbedViewData): HTMLElement {
  const root = document.createElement('div');
  root.className = 'q-md-image';
  root.contentEditable = 'false';
  root.dataset.align = data.align;
  root.dataset.kind = data.kind;

  const wrap = document.createElement('div');
  wrap.className = 'q-md-image-wrap';
  if (data.width) wrap.style.width = `${data.width}px`;

  const clip = document.createElement('div');
  clip.className = 'q-md-image-clip';

  const image = acquireMediaElement(data.url, data.kind);

  clip.append(image);
  applyImageBox(clip, image, data.crop);
  wrap.append(clip);
  root.append(wrap);
  return root;
}

export function updateAlignAction(root: HTMLElement, align: ImageAlign): void {
  const button = root.querySelector<HTMLElement>('[data-image-action="align"]');
  if (!button) return;
  button.replaceChildren(createIconElement(ALIGN_ICONS[align]));
  button.title = ALIGN_TITLES[align];
}

export function createImageChrome(align: ImageAlign, kind: MediaKind): HTMLElement[] {
  const toolbar = document.createElement('div');
  toolbar.className = 'q-floating-actions q-md-image-toolbar';
  if (kind === 'image') {
    toolbar.append(actionButton('zoom', ZoomIn, 'Открыть на весь экран'), divider());
  }
  toolbar.append(
    actionButton('align', ALIGN_ICONS[align], ALIGN_TITLES[align]),
    divider(),
    actionButton('crop', Crop, 'Кадрировать'),
    divider(),
    actionButton('code', CodeXml, 'Показать код'),
  );

  const grip = document.createElement('button');
  grip.type = 'button';
  grip.className = 'q-icon-button q-icon-button--small q-icon-button--white q-md-image-grip';
  grip.title = 'Изменить размер';
  grip.replaceChildren(createIconElement(MoveDiagonal2));

  return [toolbar, grip];
}
