import { WidgetType, type EditorView } from '@codemirror/view';
import { livePreviewConfigFacet } from '../livePreviewConfig';
import { bindWikiHover } from '../../../../modules/wikixiv';
import { getReaderQuoteViewData, renderReaderQuoteDom } from './widgetDom';

type ReaderQuoteWidgetOptions = {
  from: number;
  to: number;
  quoteText: string;
  href: string;
  refLabel: string;
};

export class ReaderQuoteWidget extends WidgetType {
  readonly from: number;
  readonly to: number;
  readonly quoteText: string;
  readonly href: string;
  readonly refLabel: string;

  constructor(options: ReaderQuoteWidgetOptions) {
    super();
    this.from = options.from;
    this.to = options.to;
    this.quoteText = options.quoteText;
    this.href = options.href;
    this.refLabel = options.refLabel;
  }

  eq(other: WidgetType): boolean {
    return other instanceof ReaderQuoteWidget
      && this.from === other.from
      && this.to === other.to
      && this.quoteText === other.quoteText
      && this.href === other.href
      && this.refLabel === other.refLabel;
  }

  private unsubscribeWikiHover: (() => void) | null = null;

  toDOM(view: EditorView): HTMLElement {
    const root = renderReaderQuoteDom({
      quoteText: this.quoteText,
      href: this.href,
      refLabel: this.refLabel,
    });

    this.unsubscribeWikiHover = bindWikiHover(root, '.q-md-reader-quote-text');

    root.addEventListener('click', (event) => {
      const target = event.target as HTMLElement | null;
      if (!target?.closest('.q-md-reader-quote-ref')) return;
      event.preventDefault();
      event.stopPropagation();
      const data = getReaderQuoteViewData(root);
      if (!data?.href) return;
      view.state.facet(livePreviewConfigFacet)?.onOpenExternalUrl(data.href);
    });

    return root;
  }

  destroy(): void {
    this.unsubscribeWikiHover?.();
    this.unsubscribeWikiHover = null;
  }

  ignoreEvent(event: Event): boolean {
    const target = event.target as HTMLElement | null;
    return !!target?.closest('.q-md-reader-quote-ref');
  }
}

