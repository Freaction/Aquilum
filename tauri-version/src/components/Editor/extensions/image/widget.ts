import { WidgetType, type EditorView } from '@codemirror/view';
import {
  sameImageCrop,
  type ImageAlign,
  type ImageCrop,
  type MediaKind,
} from '../../../../modules/docs/imageEmbeds';
import { attachmentUrl } from '../../../../modules/docs/vaultAttachments';
import { markOpenStage } from '../../../../modules/perf/openTrace';
import { bindImageEmbedEvents } from './interaction';
import {
  applyImageBox,
  cachedImageSize,
  imageUrlOf,
  imageAreaWidth,
  imageBoxAspect,
  lastImageArea,
  rememberImageSize,
  renderImageEmbedDom,
  updateAlignAction,
  type MediaElement,
} from './widgetDom';

type ImageEmbedWidgetOptions = {
  url: string;
  width: number | null;
  align: ImageAlign;
  crop: ImageCrop | null;
  kind: MediaKind;
  wikiTarget: string | null;
  workspacePath: string | null;
};

export class ImageEmbedWidget extends WidgetType {
  readonly url: string;
  readonly width: number | null;
  readonly align: ImageAlign;
  readonly crop: ImageCrop | null;
  readonly kind: MediaKind;
  readonly wikiTarget: string | null;
  readonly workspacePath: string | null;

  constructor(options: ImageEmbedWidgetOptions) {
    super();
    this.url = options.url;
    this.width = options.width;
    this.align = options.align;
    this.crop = options.crop;
    this.kind = options.kind;
    this.wikiTarget = options.wikiTarget;
    this.workspacePath = options.workspacePath;
  }

  eq(other: WidgetType): boolean {
    return other instanceof ImageEmbedWidget
      && this.url === other.url
      && this.width === other.width
      && this.align === other.align
      && this.kind === other.kind
      && this.wikiTarget === other.wikiTarget
      && sameImageCrop(this.crop, other.crop);
  }

  get estimatedHeight(): number {
    const aspect = imageBoxAspect(cachedImageSize(this.url), this.crop, this.kind);
    const width = this.width ?? lastImageArea();
    return aspect && width ? Math.round(width / aspect) : -1;
  }

  toDOM(view: EditorView): HTMLElement {
    const root = renderImageEmbedDom({
      url: this.url,
      width: this.width,
      align: this.align,
      crop: this.crop,
      kind: this.kind,
    });
    bindImageEmbedEvents(view, root);

    const clip = root.querySelector<HTMLElement>('.q-md-image-clip');
    const media = root.querySelector<MediaElement>('.q-md-image-img');
    if (clip && media && !cachedImageSize(this.url)) {
      const ready = this.kind === 'video' ? 'loadedmetadata' : 'load';
      media.addEventListener(ready, () => {
        imageAreaWidth(root);
        applyImageBox(clip, media, this.crop, rememberImageSize(media));
        view.requestMeasure();
      }, { once: true });
    }

    if (!this.url && this.wikiTarget && media) {
      void attachmentUrl(this.workspacePath, this.wikiTarget).then((url) => {
        if (url && media.isConnected) media.src = url;
        markOpenStage('images');
      });
    }

    return root;
  }

  updateDOM(dom: HTMLElement, _view: EditorView, from: ImageEmbedWidget): boolean {
    if (from.kind !== this.kind || from.wikiTarget !== this.wikiTarget) return false;
    if (from.url !== this.url && !(this.wikiTarget && !from.url)) return false;

    const wrap = dom.querySelector<HTMLElement>('.q-md-image-wrap');
    const clip = dom.querySelector<HTMLElement>('.q-md-image-clip');
    const media = dom.querySelector<MediaElement>('.q-md-image-img');
    if (!wrap || !clip || !media) return false;

    dom.dataset.align = this.align;
    updateAlignAction(dom, this.align);
    wrap.style.width = this.width ? `${this.width}px` : '';
    if (this.url && imageUrlOf(media) !== this.url) media.src = this.url;
    if (!sameImageCrop(from.crop, this.crop)) applyImageBox(clip, media, this.crop);
    return true;
  }

  destroy(dom: HTMLElement): void {
    const media = dom.querySelector<MediaElement>('.q-md-image-img');
    if (media instanceof HTMLVideoElement) media.pause();
  }

  ignoreEvent(event: Event): boolean {
    const target = event.target as HTMLElement | null;
    return !!target?.closest('.q-md-image-clip, .q-floating-actions, .q-md-image-grip');
  }
}
