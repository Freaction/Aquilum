import { EditorSelection } from '@codemirror/state';
import { WidgetType, type EditorView } from '@codemirror/view';
import type { WikiLinkResolver } from '../../../../modules/links';
import { formatPages } from '../../../../modules/docs/bookProgress';
import { subscribeBookPageRuntime } from '../../../../modules/docs/bookPageRuntime';
import { bindWikiHover } from '../../../../modules/wikixiv';
import { livePreviewConfigFacet } from '../livePreviewConfig';
import type { BookCalloutGroup } from './constructs';
import { bookCalloutSignature, resolveCalloutData, viewDataFromEntry, viewDataFromModel } from './linkedCache';
import { parseBookCalloutBlock } from './model';
import {
  getBookCalloutViewData,
  patchBookCalloutDom,
  renderBookCalloutDom,
  renderBookCalloutGroupDom,
  renderBookCalloutPendingDom,
  type BookCalloutViewData,
} from './widgetDom';

type Options = {
  group: BookCalloutGroup;
  workspacePath: string | null;
  resolveWikiLinks: WikiLinkResolver;
};

const UNTITLED = { title: 'Книга', wikiTarget: null, author: '', cover: '', filePath: '' };

function groupViewData(
  group: BookCalloutGroup,
  workspacePath: string | null,
): Array<BookCalloutViewData | null> {
  return group.items.map((span) => {
    const model = parseBookCalloutBlock(span.text);
    return model
      ? resolveCalloutData(model, workspacePath)
      : viewDataFromModel(UNTITLED, workspacePath);
  });
}

export class BookCalloutGroupWidget extends WidgetType {
  private readonly cleanups: Array<() => void> = [];
  private readonly signature: string;

  constructor(private readonly options: Options) {
    super();
    this.signature = groupViewData(options.group, options.workspacePath)
      .map((data) => bookCalloutSignature(data))
      .join('\u0001');
  }

  eq(other: WidgetType): boolean {
    return other instanceof BookCalloutGroupWidget
      && this.signature === other.signature
      && this.options.workspacePath === other.options.workspacePath
      && this.options.group.from === other.options.group.from
      && this.options.group.to === other.options.group.to
      && this.options.group.items.every((item, index) => item.text === other.options.group.items[index]?.text);
  }

  toDOM(view: EditorView): HTMLElement {
    const groupRoot = renderBookCalloutGroupDom();
    const rows = groupViewData(this.options.group, this.options.workspacePath);

    this.options.group.items.forEach((span, index) => {
      const data = rows[index] ?? null;
      const row = data ? renderBookCalloutDom(data) : renderBookCalloutPendingDom();
      row.dataset.sourceFrom = String(span.from);
      groupRoot.append(row);
      if (!data) return;

      this.cleanups.push(bindWikiHover(row, '.q-md-book-callout-title'));
      if (data.bookPagePath) {
        this.cleanups.push(this.bindProgress(view, row, data.bookPagePath));
      }
    });

    groupRoot.addEventListener('mousedown', (event) => this.handleClick(view, event));
    return groupRoot;
  }

  private bindProgress(view: EditorView, row: HTMLElement, bookPagePath: string): () => void {
    return subscribeBookPageRuntime(bookPagePath, (entry) => {
      if (!row.isConnected) return;
      const current = getBookCalloutViewData(row);
      if (!current) return;
      patchBookCalloutDom(row, viewDataFromEntry(current, bookPagePath, entry, this.options.workspacePath));
      view.requestMeasure();
    });
  }

  private handleClick(view: EditorView, event: MouseEvent): void {
    const target = event.target as HTMLElement | null;
    const row = target?.closest('.q-md-book-callout') as HTMLElement | null;
    if (!row) return;
    const config = view.state.facet(livePreviewConfigFacet);
    const data = getBookCalloutViewData(row);
    const title = target?.closest('.q-md-book-callout-title--link');
    if (title && data?.wikiTarget) {
      event.preventDefault();
      event.stopPropagation();
      config?.onOpenWikiLink(data.wikiTarget, event.ctrlKey || event.metaKey ? 'new-tab' : 'current');
      return;
    }
    const read = target?.closest('.q-md-book-callout-read') as HTMLButtonElement | null;
    if (read) {
      event.preventDefault();
      event.stopPropagation();
      if (!read.disabled && data?.canRead && data.filePath) {
        config?.onReadBook({
          bookFile: data.filePath,
          title: data.title,
          pagesFm: data.pages ? formatPages(data.pages.current, data.pages.total) : undefined,
          bookPagePath: data.bookPagePath,
        });
      }
      return;
    }
    const pos = Number(row.dataset.sourceFrom);
    if (Number.isFinite(pos)) view.dispatch({ selection: EditorSelection.cursor(pos), scrollIntoView: true });
  }

  destroy(): void {
    for (const cleanup of this.cleanups.splice(0)) cleanup();
  }

  ignoreEvent(event: Event): boolean {
    return event.type === 'mousedown' || event.type === 'click';
  }
}
