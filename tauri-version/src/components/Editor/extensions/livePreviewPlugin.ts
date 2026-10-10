import { syntaxTree } from '@codemirror/language';
import { type EditorState, type Extension, RangeSetBuilder, StateEffect } from '@codemirror/state';
import {
  Decoration,
  type DecorationSet,
  EditorView,
  ViewPlugin,
  type ViewUpdate,
} from '@codemirror/view';
import type { SyntaxNode, Tree } from '@lezer/common';
import { stateFrontmatterRange, setFrontmatterExpanded } from './frontmatterUi';
import { livePreviewConfigFacet, previewCaret, type LivePreviewConfig } from './livePreviewConfig';
import { markOpenStage } from '../../../modules/perf/openTrace';
import { livePreviewWidgetExtension } from './livePreviewWidgets';
import { ensureEditorTree, visibleTreeRanges } from './ensureEditorTree';
import {
  collectEditablePreviewSpans,
  shouldRevealEditablePreviewMarker,
} from './livePreviewEditableSpans';
import { collectBlockquoteDecorations } from './blockquotePreview';
import { revealTarget } from './livePreviewReveal';
import { shouldRevealSyntax } from './livePreviewVisibility';
import { syntaxMarkerPresentation } from './livePreviewSyntax';
import { isFenceMark } from './codeBlock/blocks';
import { hasAncestorNamed } from './syntaxAncestor';

const hrLineDecoration = Decoration.line({ class: 'q-md-hr-line-cm' });
const hidden = Decoration.mark({ class: 'q-md-hidden-syntax' });
const transparent = Decoration.mark({ class: 'q-md-transparent-syntax' });
const unresolved = Decoration.mark({ class: 'q-wiki-link--unresolved' });
const collapsed = Decoration.replace({});
const refreshWikiLinks = StateEffect.define<void>();
export const invalidateWikiLinks = StateEffect.define<void>();

function wikiHasTarget(node: SyntaxNode, slice: (from: number, to: number) => string): boolean {
  const target = node.getChild('WikiLinkTarget');
  return Boolean(target && slice(target.from, target.to).trim());
}

type Slice = (from: number, to: number) => string;

function isLinkHref(node: SyntaxNode, slice: Slice): boolean {
  const opener = node.prevSibling;
  return Boolean(
    opener && opener.name === 'LinkMark' && slice(opener.from, opener.to) === '(',
  );
}

function belongsToInlineLink(node: SyntaxNode, slice: Slice): boolean {
  if (node.name !== 'LinkMark' && node.name !== 'URL') return false;
  const link = node.parent;
  if (!link || link.name !== 'Link') return false;
  for (let child = link.firstChild; child; child = child.nextSibling) {
    if (child.name === 'LinkMark' && slice(child.from, child.to) === '(') return true;
  }
  return false;
}

function selectionInsideWikiLink(state: EditorState): boolean {
  return hasAncestorNamed(syntaxTree(state).resolveInner(state.selection.main.head, -1), 'WikiLink');
}

export function mergeCollapseRanges(
  ranges: { from: number; to: number }[],
): { from: number; to: number }[] {
  if (ranges.length === 0) return [];
  ranges.sort((a, b) => a.from - b.from || a.to - b.to);
  const out: { from: number; to: number }[] = [];
  let from = ranges[0]!.from;
  let to = ranges[0]!.to;
  for (let i = 1; i < ranges.length; i += 1) {
    const next = ranges[i]!;
    if (next.from <= to) {
      to = Math.max(to, next.to);
      continue;
    }
    if (to > from) out.push({ from, to });
    from = next.from;
    to = next.to;
  }
  if (to > from) out.push({ from, to });
  return out;
}

export function collectCollapseRanges(
  state: EditorState,
  tree: Tree,
  ranges: readonly { from: number; to: number }[] = [
    { from: 0, to: Math.min(tree.length, state.doc.length) },
  ],
  editableSpans = collectEditablePreviewSpans(state.doc),
): { from: number; to: number }[] {
  const head = previewCaret(state);
  const charAt = (pos: number) => state.doc.sliceString(pos, pos + 1);
  const slice = (a: number, b: number) => state.doc.sliceString(a, b);
  const fmRange = stateFrontmatterRange(state);
  const collapse: { from: number; to: number }[] = [];

  for (const range of ranges) {
    tree.iterate({
      from: range.from,
      to: Math.min(range.to, tree.length),
      enter(node) {
        if (syntaxMarkerPresentation(node.name) !== 'collapsed') return;
        if (node.name === 'URL' && !isLinkHref(node.node, slice)) return;
        if (
          fmRange
          && node.from < fmRange.to
          && node.to > fmRange.from
          && !belongsToInlineLink(node.node, slice)
        ) {
          return;
        }
        if (
          (node.name === 'WikiLinkMark' || node.name === 'WikiLinkAliasMark')
          && node.node.parent?.name === 'WikiLink'
          && !wikiHasTarget(node.node.parent, slice)
        ) {
          return;
        }
        if (shouldRevealEditablePreviewMarker(editableSpans, head, node.from, node.name)) return;
        const hit = revealTarget(node.node, charAt);
        if (shouldRevealSyntax(state.doc, head, hit.from, hit.to)) return;
        if (hit.hideTo > node.from) collapse.push({ from: node.from, to: hit.hideTo });
      },
    });

    tree.iterate({
      from: range.from,
      to: Math.min(range.to, tree.length),
      enter(node) {
        if (node.name !== 'WikiLink') return;
        if (fmRange && node.from < fmRange.to && node.to > fmRange.from) return;
        const target = node.node.getChild('WikiLinkTarget');
        const alias = node.node.getChild('WikiLinkAlias');
        if (!target || !alias || target.to >= alias.from) return;
        if (shouldRevealEditablePreviewMarker(editableSpans, head, node.from)) return;
        if (shouldRevealSyntax(state.doc, head, node.from, node.to)) return;
        collapse.push({ from: target.from, to: alias.from });
      },
    });
  }

  return mergeCollapseRanges(collapse);
}

function buildLivePreviewDecorations(
  view: EditorView,
  resolutions: ReadonlyMap<string, boolean>,
  allowParse = true,
) {
  const decs: { from: number; to: number; dec: Decoration }[] = [];
  const targets = new Set<string>();
  const { state } = view;
  const head = previewCaret(state);
  const charAt = (pos: number) => state.sliceDoc(pos, pos + 1);
  const slice = (from: number, to: number) => state.sliceDoc(from, to);
  const fmRange = stateFrontmatterRange(state);
  const tree = ensureEditorTree(view, allowParse);
  const editableSpans = collectEditablePreviewSpans(state.doc);

  const inFrontmatter = (from: number, to: number) =>
    Boolean(fmRange && from < fmRange.to && to > fmRange.from);

  const enter = (node: { name: string; from: number; to: number; node: SyntaxNode }) => {
    if (inFrontmatter(node.from, node.to)) return;

    if (node.name === 'WikiLink') {
      const target = node.node.getChild('WikiLinkTarget');
      if (!target) return;
      const text = slice(target.from, target.to).trim();
      if (!text) return;
      targets.add(text);
      if (resolutions.get(text) === false) {
        const alias = node.node.getChild('WikiLinkAlias');
        const revealed = shouldRevealSyntax(state.doc, head, node.from, node.to);
        if (revealed || !alias) {
          decs.push({ from: target.from, to: target.to, dec: unresolved });
        }
        if (alias) {
          decs.push({ from: alias.from, to: alias.to, dec: unresolved });
        }
      }
      return;
    }

    const presentation = syntaxMarkerPresentation(node.name);
    if (!presentation || presentation === 'collapsed') return;
    if (isFenceMark(node)) return;
    if (shouldRevealEditablePreviewMarker(editableSpans, head, node.from, node.name)) return;

    const hit = revealTarget(node.node, charAt);
    if (shouldRevealSyntax(state.doc, head, hit.from, hit.to)) return;

    if (presentation === 'horizontal-rule') {
      decs.push({ from: node.from, to: node.from, dec: hrLineDecoration });
    }
    decs.push({
      from: node.from,
      to: hit.hideTo,
      dec: presentation === 'hidden' ? hidden : transparent,
    });
  };

  const visible = visibleTreeRanges(view, tree.length);
  for (const range of visible) {
    tree.iterate({
      from: range.from,
      to: Math.min(range.to, tree.length),
      enter,
    });
  }

  for (const range of collectCollapseRanges(state, tree, visible, editableSpans)) {
    decs.push({ from: range.from, to: range.to, dec: collapsed });
  }

  for (const dec of collectBlockquoteDecorations(state, tree, visible)) {
    decs.push(dec);
  }

  decs.sort((a, b) => a.from - b.from || a.to - b.to);
  const builder = new RangeSetBuilder<Decoration>();
  for (const d of decs) builder.add(d.from, d.to, d.dec);
  return {
    decorations: builder.finish(),
    targets: [...targets],
  };
}

function livePreviewPlugin(config: LivePreviewConfig) {
  return ViewPlugin.fromClass(class {
    decorations: DecorationSet = Decoration.none;
    private readonly pending = new Set<string>();
    private readonly resolutions = new Map<string, boolean>();
    private generation = 0;

    constructor(view: EditorView) {
      this.rebuild(view);
    }

    update(update: ViewUpdate) {
      const invalidated = update.transactions.some((transaction) =>
        transaction.effects.some((effect) => effect.is(invalidateWikiLinks)),
      );
      if (invalidated) {
        this.generation += 1;
        this.pending.clear();
        this.resolutions.clear();
        this.rebuild(update.view);
        return;
      }

      const refreshed = update.transactions.some((transaction) =>
        transaction.effects.some((effect) => effect.is(refreshWikiLinks)),
      );
      const fmToggled = update.transactions.some((transaction) =>
        transaction.effects.some((effect) => effect.is(setFrontmatterExpanded)),
      );
      const treeChanged = syntaxTree(update.state) !== syntaxTree(update.startState);
      if (
        update.docChanged
        || update.selectionSet
        || update.viewportChanged
        || refreshed
        || fmToggled
        || treeChanged
      ) {
        this.rebuild(update.view, update.docChanged || treeChanged || refreshed || fmToggled);
      }
    }

    destroy() {
      this.generation += 1;
    }

    private rebuild(view: EditorView, allowParse = true) {
      try {
        const result = buildLivePreviewDecorations(view, this.resolutions, allowParse);
        this.decorations = result.decorations;
        const missing = result.targets.filter((target) =>
          !this.resolutions.has(target) && !this.pending.has(target),
        );
        if (missing.length > 0) this.resolve(view, missing);
      } catch (error) {
        console.error('live preview rebuild failed', error);
      }
    }

    private resolve(view: EditorView, targets: string[]) {
      targets.forEach((target) => this.pending.add(target));
      const generation = this.generation;
      void config.resolveWikiLinks(targets).then((result) => {
        if (generation !== this.generation) return;
        let changed = false;
        targets.forEach((target, index) => {
          this.pending.delete(target);
          const next = result.paths[index] != null || !result.complete;
          if (this.resolutions.get(target) !== next) {
            this.resolutions.set(target, next);
            changed = true;
          }
        });
        if (this.pending.size === 0) markOpenStage('links');
        if (
          changed
          && result.complete
          && !selectionInsideWikiLink(view.state)
        ) {
          view.dispatch({ effects: refreshWikiLinks.of() });
        }
      }).catch((error) => {
        targets.forEach((target) => this.pending.delete(target));
        console.error(error);
      });
    }
  }, { decorations: (value) => value.decorations });
}

export function livePreviewExtension(config: LivePreviewConfig): Extension {
  return [
    livePreviewConfigFacet.of(config),
    livePreviewPlugin(config),
    livePreviewWidgetExtension(),
  ];
}
