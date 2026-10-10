import { WidgetType, type EditorView } from '@codemirror/view';
import { takeTableCellFocus } from './apply';
import type { TableModel } from './model';
import { renderTableWidgetDom } from './widgetDom';
import { bindWikiHover, refreshWikiHover, unbindWikiHover } from '../../../../modules/wikixiv';
import { livePreviewConfigFacet } from '../livePreviewConfig';
import { WidgetSession } from './widgetSession';
import { bindWidgetSession, syncWidgetSession, unbindWidgetSession } from './widgetSessionRegistry';

const TABLE_HIT_TARGET =
    '.q-md-table-scroll, .q-md-table-layout, .q-md-table-add-row, .q-md-table-add-col, .q-md-table-cell-field, .cm-editor';

const WIDGET_CHROME_PX = 80;

interface TableWidgetOptions {
    from: number;
    contentTo: number;
    blockTo: number;
    model: TableModel;
    text: string;
}

export class TableWidget extends WidgetType {
    readonly from: number;
    readonly contentTo: number;
    readonly blockTo: number;
    readonly model: TableModel;
    readonly text: string;

    constructor(options: TableWidgetOptions) {
        super();
        this.from = options.from;
        this.contentTo = options.contentTo;
        this.blockTo = options.blockTo;
        this.model = options.model;
        this.text = options.text;
    }

    eq(other: WidgetType): boolean {
        return other instanceof TableWidget
            && this.text === other.text
            && this.from === other.from
            && this.contentTo === other.contentTo;
    }

    updateDOM(dom: HTMLElement): boolean {
        const updated = syncWidgetSession(dom, {
            from: this.from,
            contentTo: this.contentTo,
            blockTo: this.blockTo,
        }, this.model);
        refreshWikiHover(dom);
        return updated;
    }

    toDOM(view: EditorView): HTMLElement {
        const root = renderTableWidgetDom(this.model);
        bindWikiHover(root, '.q-md-table-cell-wrapper');

        const session = new WidgetSession(
            view,
            { from: this.from, contentTo: this.contentTo, blockTo: this.blockTo },
            this.model,
            root,
        );
        bindWidgetSession(root, session);
        root.addEventListener('mousedown', (event) => {
            if (event.button !== 0) return;
            if (!(event.target as HTMLElement | null)?.closest('.q-md-table-link')) return;
            event.preventDefault();
            event.stopImmediatePropagation();
        });
        root.addEventListener('click', (event) => {
            const link = (event.target as HTMLElement | null)?.closest<HTMLElement>('.q-md-table-link');
            if (!link || event.button !== 0) return;
            event.preventDefault();
            event.stopPropagation();
            const config = view.state.facet(livePreviewConfigFacet);
            if (link.dataset.wikiTarget) config?.onOpenWikiLink(
                link.dataset.wikiTarget,
                event.ctrlKey || event.metaKey ? 'new-tab' : 'current',
            );
            else if (link.dataset.externalUrl) config?.onOpenExternalUrl(link.dataset.externalUrl);
        });
        root.addEventListener('mousedown', (event) => session.onMouseDown(event));
        root.addEventListener('keydown', (event) => session.onKeyDown(event));
        root.addEventListener('contextmenu', (event) => session.onContextMenu(event));
        root.tabIndex = -1;

        const pending = takeTableCellFocus(view);
        if (pending && pending.tableFrom === this.from) {
            requestAnimationFrame(() => {
                if (!session.isDisposed) {
                    session.activateCell({ row: pending.row, col: pending.col });
                }
            });
        }

        return root;
    }

    destroy(dom: HTMLElement): void {
        unbindWidgetSession(dom);
        unbindWikiHover(dom);
    }

    get estimatedHeight(): number {
        let rows = 0;
        for (const height of this.model.rowHeights) rows += height;
        return Math.max(48, Math.ceil(rows * 1.2) + WIDGET_CHROME_PX);
    }

    ignoreEvent(event: Event): boolean {
        const target = event.target as HTMLElement | null;
        return !!target?.closest(TABLE_HIT_TARGET);
    }
}

