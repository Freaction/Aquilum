import { Facet, type EditorState, type Line, type StateEffectType } from '@codemirror/state';
import { syntaxTree } from '@codemirror/language';
import {
  Decoration,
  type DecorationSet,
  EditorView,
  ViewPlugin,
  type ViewUpdate,
} from '@codemirror/view';
import type { SyntaxNodeRef, Tree } from '@lezer/common';
import { ensureEditorTree, visibleTreeRanges } from './ensureEditorTree';

export interface ViewportScanContext {
  view: EditorView;
  state: EditorState;
  tree: Tree | null;
}

interface ViewportTriggers {
  doc?: boolean;
  tree?: boolean;
  selection?: boolean;
  viewport?: boolean;
  effects?: readonly StateEffectType<unknown>[];
}

export interface ViewportCollector<T = unknown> {
  id: string;
  triggers: ViewportTriggers;
  scan: 'tree' | 'lines' | 'both';
  begin(context: ViewportScanContext): T;
  enterNode?(node: SyntaxNodeRef, accumulator: T, context: ViewportScanContext): void;
  enterLine?(line: Line, accumulator: T, context: ViewportScanContext): void;
  finish(accumulator: T, context: ViewportScanContext): DecorationSet;
}

const viewportCollectors = Facet.define<ViewportCollector<unknown>>();

function effectFired(update: ViewUpdate, types: readonly StateEffectType<unknown>[]): boolean {
  return types.some((type) => update.transactions.some(
    (transaction) => transaction.effects.some((effect) => effect.is(type)),
  ));
}

function triggered(collector: ViewportCollector<unknown>, update: ViewUpdate, treeChanged: boolean): boolean {
  const { triggers } = collector;
  if (triggers.doc && update.docChanged) return true;
  if (triggers.tree && treeChanged) return true;
  if (triggers.selection && update.selectionSet) return true;
  if (triggers.viewport && update.viewportChanged) return true;
  if (triggers.effects && effectFired(update, triggers.effects)) return true;
  return false;
}

class ViewportScanDriver {
  private sets = new Map<string, DecorationSet>();

  constructor(view: EditorView) {
    this.run(view, view.state.facet(viewportCollectors), true);
  }

  update(update: ViewUpdate): void {
    if (update.docChanged) {
      for (const [id, set] of this.sets) this.sets.set(id, set.map(update.changes));
    }

    const collectors = update.view.state.facet(viewportCollectors);
    const treeChanged = syntaxTree(update.state) !== syntaxTree(update.startState);
    const active = collectors.filter((collector) => triggered(collector, update, treeChanged));
    if (active.length === 0) return;

    const allowParse = update.docChanged
      || treeChanged
      || active.some((collector) => collector.triggers.effects
        && effectFired(update, collector.triggers.effects));

    this.run(update.view, active, allowParse);
  }

  decorationsFor(id: string): DecorationSet {
    return this.sets.get(id) ?? Decoration.none;
  }

  private run(
    view: EditorView,
    collectors: readonly ViewportCollector<unknown>[],
    allowParse: boolean,
  ): void {
    if (collectors.length === 0) return;

    const wantsTree = collectors.some((collector) => collector.scan !== 'lines');
    const tree = wantsTree ? ensureEditorTree(view, allowParse) : null;
    const context: ViewportScanContext = { view, state: view.state, tree };

    const accumulators = new Map<string, unknown>();
    for (const collector of collectors) accumulators.set(collector.id, collector.begin(context));

    if (tree) {
      const nodeCollectors = collectors.filter((collector) => collector.enterNode);
      if (nodeCollectors.length > 0) {
        for (const range of visibleTreeRanges(view, tree.length)) {
          tree.iterate({
            from: range.from,
            to: Math.min(range.to, tree.length),
            enter: (node) => {
              for (const collector of nodeCollectors) {
                collector.enterNode?.(node, accumulators.get(collector.id), context);
              }
            },
          });
        }
      }
    }

    const lineCollectors = collectors.filter((collector) => collector.enterLine);
    if (lineCollectors.length > 0) {
      const { doc } = view.state;
      for (const range of view.visibleRanges) {
        let pos = range.from;
        while (pos <= range.to) {
          const line = doc.lineAt(pos);
          for (const collector of lineCollectors) {
            collector.enterLine?.(line, accumulators.get(collector.id), context);
          }
          if (line.to >= doc.length) break;
          pos = line.to + 1;
        }
      }
    }

    for (const collector of collectors) {
      this.sets.set(collector.id, collector.finish(accumulators.get(collector.id), context));
    }
  }
}

const viewportScanPlugin = ViewPlugin.fromClass(ViewportScanDriver);

export function viewportCollector<T>(collector: ViewportCollector<T>) {
  return [
    viewportScanPlugin,
    viewportCollectors.of(collector as ViewportCollector<unknown>),
    EditorView.decorations.of(
      (view) => view.plugin(viewportScanPlugin)?.decorationsFor(collector.id) ?? Decoration.none,
    ),
  ];
}
