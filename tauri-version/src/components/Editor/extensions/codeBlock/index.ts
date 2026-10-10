import { syntaxTree } from '@codemirror/language';
import { Decoration, ViewPlugin, type DecorationSet, type EditorView, type ViewUpdate } from '@codemirror/view';
import { ensureEditorTree, visibleTreeRanges } from '../ensureEditorTree';
import { codeBlocks } from './blocks';
import { codeBlockDecorations, codeBlockLineDecorationsFacet } from './decorations';
import { CodeGrammarLoader } from './grammars';
import { codeHighlighting } from './highlight';
import { codeBlockTheme } from './theme';

const codeBlockPlugin = ViewPlugin.fromClass(class {
  decorations: DecorationSet;
  private readonly grammars = new CodeGrammarLoader();

  constructor(view: EditorView) {
    this.decorations = this.rebuild(view);
  }

  update(update: ViewUpdate): void {
    const treeChanged = syntaxTree(update.state) !== syntaxTree(update.startState);
    const stylingChanged = update.state.facet(codeBlockLineDecorationsFacet) !== update.startState.facet(codeBlockLineDecorationsFacet);
    if (!update.docChanged && !update.viewportChanged && !treeChanged && !stylingChanged) return;
    this.decorations = this.rebuild(update.view, update.docChanged || treeChanged);
  }

  destroy(): void {
    this.grammars.dispose();
  }

  private rebuild(view: EditorView, allowParse = true): DecorationSet {
    const tree = ensureEditorTree(view, allowParse);
    const blocks = codeBlocks(tree, view.state.doc, visibleTreeRanges(view, tree.length));
    this.grammars.sync(view, tree, blocks);
    return view.state.facet(codeBlockLineDecorationsFacet) ? codeBlockDecorations(view.state.doc, blocks) : Decoration.none;
  }
}, {
  decorations: (plugin) => plugin.decorations,
});

export const codeBlockExtension = [codeBlockPlugin, codeHighlighting, codeBlockTheme];
