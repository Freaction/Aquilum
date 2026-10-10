import { forceParsing } from '@codemirror/language';
import type { Text } from '@codemirror/state';
import type { EditorView } from '@codemirror/view';
import type { Tree } from '@lezer/common';
import type { CodeBlockShape } from './blocks';
import { fenceLanguage } from './languages';

const MOUNT_PARSE_BUDGET_MS = 100;

export function isGrammarMounted(tree: Tree, doc: Text, block: CodeBlockShape): boolean {
  const firstLine = doc.lineAt(block.from);
  if (firstLine.to >= block.end) return true;
  return tree.resolveInner(firstLine.to + 1, 1).name !== 'CodeText';
}

export class CodeGrammarLoader {
  private readonly requested = new Set<string>();
  private forcedTree: Tree | null = null;
  private disposed = false;

  dispose(): void {
    this.disposed = true;
  }

  sync(view: EditorView, tree: Tree, blocks: readonly CodeBlockShape[]): void {
    for (const block of blocks) {
      const language = fenceLanguage(block.info);
      if (!language) continue;
      if (!language.support) {
        this.request(view, language.name, language.load());
      } else if (!isGrammarMounted(tree, view.state.doc, block)) {
        this.remount(view, tree);
      }
    }
  }

  private request(view: EditorView, name: string, loading: Promise<unknown>): void {
    if (this.requested.has(name)) return;
    this.requested.add(name);
    void loading
      .then(() => this.remount(view, null))
      .catch(() => this.requested.delete(name));
  }

  private remount(view: EditorView, parsedTree: Tree | null): void {
    if (this.disposed || (parsedTree !== null && this.forcedTree === parsedTree)) return;
    this.forcedTree = parsedTree;
    queueMicrotask(() => {
      if (!this.disposed) forceParsing(view, view.viewport.to, MOUNT_PARSE_BUDGET_MS);
    });
  }
}
