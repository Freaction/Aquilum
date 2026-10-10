import { StateEffect, type Extension } from '@codemirror/state';
import { ViewPlugin, type EditorView, type ViewUpdate } from '@codemirror/view';
import { markOpenStage } from '../../../../modules/perf/openTrace';
import { livePreviewConfigFacet } from '../livePreviewConfig';
import { PrefetchScheduler, type PrefetchDelays } from '../prefetchScheduler';
import { groupAdjacentBookCallouts } from './constructs';
import { prefetchLinkedCallouts } from './linkedCache';
import { parseBookCalloutBlock } from './model';

export const bookCalloutDataReady = StateEffect.define<null>();

const DELAYS: PrefetchDelays = { editMs: 40, revisionMs: 40 };

const COVER_WARMUP_TIMEOUT_MS = 1500;

function warmCovers(urls: string[]): Promise<void> {
  if (urls.length === 0) return Promise.resolve();
  return new Promise((resolve) => {
    let left = urls.length;
    const timer = setTimeout(resolve, COVER_WARMUP_TIMEOUT_MS);
    const settle = () => {
      left -= 1;
      if (left > 0) return;
      clearTimeout(timer);
      resolve();
    };
    for (const url of urls) {
      const image = new Image();
      image.onload = settle;
      image.onerror = settle;
      image.src = url;
    }
  });
}

function linkedModels(view: EditorView) {
  return groupAdjacentBookCallouts(view.state.doc)
    .flatMap((group) => group.items)
    .map((span) => parseBookCalloutBlock(span.text))
    .filter((model) => Boolean(model?.wikiTarget))
    .map((model) => model!);
}

class BookCalloutPrefetcher {
  private readonly scheduler: PrefetchScheduler;

  constructor(private readonly view: EditorView) {
    this.scheduler = new PrefetchScheduler(view, DELAYS, (refresh) => this.run(refresh));
  }

  update(update: ViewUpdate): void {
    if (update.docChanged) this.scheduler.schedule(false);
  }

  destroy(): void {
    this.scheduler.destroy();
  }

  private async run(refresh: boolean): Promise<void> {
    const config = this.view.state.facet(livePreviewConfigFacet);
    const workspacePath = config?.workspacePath;
    if (this.scheduler.destroyed || !config || !workspacePath) return;
    const models = linkedModels(this.view);
    if (models.length === 0) return;

    const result = await prefetchLinkedCallouts(
      models,
      workspacePath,
      config.resolveWikiLinks,
      refresh,
    );
    if (result.changed) {
      await warmCovers(result.covers);
      if (!this.scheduler.destroyed) this.view.dispatch({ effects: bookCalloutDataReady.of(null) });
    }
    markOpenStage('callouts');
  }
}

export function bookCalloutPrefetchExtension(): Extension {
  return ViewPlugin.fromClass(BookCalloutPrefetcher);
}
