import { EditorSelection } from '@codemirror/state';
import { WidgetType, type EditorView } from '@codemirror/view';
import { TASK_CHECKBOX_CLASS } from '../../../Common/taskCheckbox';
import { toggleTaskInFile } from '../../../../modules/documents/toggleTaskInFile';
import { livePreviewConfigFacet } from '../livePreviewConfig';
import { DATAVIEW_SELECTORS } from './constants';
import { dataviewDataReady } from './prefetch';
import {
  cachedDataviewResult,
  dataviewBlockHeight,
  dataviewResultSignature,
  rememberDataviewBlockHeight,
  toggleCachedDataviewTask,
} from './store';
import { dataviewLinkTarget, dataviewTaskAt, renderDataviewDom } from './widgetDom';

type Options = {
  query: string;
  notePath: string;
};

const CHECK_SELECTOR = `.${TASK_CHECKBOX_CLASS}`;

export class DataviewWidget extends WidgetType {
  private readonly signature: string;

  constructor(private readonly options: Options) {
    super();
    this.signature = dataviewResultSignature(options.notePath, options.query);
  }

  eq(other: WidgetType): boolean {
    return other instanceof DataviewWidget
      && this.options.query === other.options.query
      && this.options.notePath === other.options.notePath
      && this.signature === other.signature;
  }

  get estimatedHeight(): number {
    return dataviewBlockHeight(this.options.notePath, this.options.query);
  }

  toDOM(view: EditorView): HTMLElement {
    const { notePath, query } = this.options;
    const root = renderDataviewDom(
      cachedDataviewResult(notePath, query),
      dataviewBlockHeight(notePath, query),
    );

    view.requestMeasure({
      read: () => root.getBoundingClientRect().height,
      write: (height) => rememberDataviewBlockHeight(notePath, query, Math.round(height)),
    });

    root.addEventListener('mousedown', (event) => {
      if (!(event.target as HTMLElement | null)?.closest?.(DATAVIEW_SELECTORS.card)) return;
      const config = view.state.facet(livePreviewConfigFacet);
      const task = dataviewTaskAt(event.target);
      if (task && config?.workspacePath) {
        event.preventDefault();
        event.stopPropagation();
        const box = (event.target as HTMLElement | null)?.closest?.(CHECK_SELECTOR);
        box?.classList.toggle(`${TASK_CHECKBOX_CLASS}--done`);
        if (toggleCachedDataviewTask(task.target, task.line)) {
          view.dispatch({ effects: dataviewDataReady.of(null) });
        }
        void toggleTaskInFile(config.workspacePath, task.target, task.line)
          .catch((error) => console.error('Failed to toggle task', error));
        return;
      }
      const target = dataviewLinkTarget(event.target);
      if (target) {
        event.preventDefault();
        event.stopPropagation();
        config?.onOpenWikiLink(target, event.ctrlKey || event.metaKey ? 'new-tab' : 'current');
        return;
      }
      event.preventDefault();
      view.dispatch({
        selection: EditorSelection.cursor(view.posAtDOM(root)),
        scrollIntoView: true,
      });
      view.focus();
    });

    return root;
  }

  ignoreEvent(event: Event): boolean {
    if (!(event.target as HTMLElement | null)?.closest?.(DATAVIEW_SELECTORS.card)) return false;
    return event.type === 'mousedown' || event.type === 'click';
  }
}
