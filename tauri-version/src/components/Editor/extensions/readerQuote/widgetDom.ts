import { formatReaderQuoteRef } from '../../../../modules/docs/bookQuotes';

type ReaderQuoteViewData = {
  quoteText: string;
  href: string;
  refLabel: string;
};

const viewDataByRoot = new WeakMap<HTMLElement, ReaderQuoteViewData>();

export function getReaderQuoteViewData(root: HTMLElement): ReaderQuoteViewData | undefined {
  return viewDataByRoot.get(root);
}

function storeReaderQuoteViewData(root: HTMLElement, data: ReaderQuoteViewData): void {
  viewDataByRoot.set(root, data);
}

export function renderReaderQuoteDom(data: ReaderQuoteViewData): HTMLElement {
  const root = document.createElement('div');
  root.className = 'q-md-blockquote q-md-reader-quote';
  root.contentEditable = 'false';

  const text = document.createElement('span');
  text.className = 'q-md-reader-quote-text';
  text.textContent = data.quoteText;

  const ref = document.createElement('button');
  ref.type = 'button';
  ref.className = 'q-md-reader-quote-ref';
  ref.textContent = formatReaderQuoteRef(Number(data.refLabel));
  ref.title = 'Открыть в книге';

  root.append(text, document.createTextNode('\u00a0'), ref);
  storeReaderQuoteViewData(root, data);
  return root;
}
