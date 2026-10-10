import { StateField, type Extension } from '@codemirror/state';
import { Decoration, EditorView, WidgetType, type DecorationSet } from '@codemirror/view';
import { syntaxTree, syntaxTreeAvailable } from '@codemirror/language';
import { Check, Copy } from 'lucide';
import { createIconElement } from '../../../../Common/iconElement';
import { t } from '../../../../../i18n';
import type { PluginSettings } from '../../../../../modules/settings';
import { codeBlocks, type CodeBlockShape } from '../../codeBlock/blocks';
import { codeBlockDecorations, codeBlockLineDecorationsFacet } from '../../codeBlock/decorations';
import { editorMarkdownSupport } from '../../markdownConfig';
import { previewCaret, revealAtCaretFacet } from '../../livePreviewConfig';
import { parseInfo } from './parseInfo';

class CodeHeader extends WidgetType {
  private readonly timers = new WeakMap<HTMLElement, ReturnType<typeof setTimeout>>();
  constructor(readonly caption: string, readonly body: string, readonly copy: boolean) { super(); }
  eq(other: CodeHeader) { return this.caption === other.caption && this.body === other.body && this.copy === other.copy; }
  toDOM() {
    const root = document.createElement('div');
    root.className = this.caption ? 'q-code-styler-header' : 'q-code-styler-floating';
    if (this.caption) {
      const label = document.createElement('span');
      label.textContent = this.caption;
      root.append(label);
    }
    if (this.copy) {
      const button = document.createElement('button');
      button.type = 'button';
      button.className = 'q-button q-md-code-copy';
      button.setAttribute('aria-label', t('plugins.codeStyler.copy'));
      button.append(createIconElement(Copy));
      const status = document.createElement('span');
      status.className = 'q-code-styler-status';
      status.setAttribute('aria-live', 'polite');
      button.addEventListener('click', async () => {
        try {
          await navigator.clipboard.writeText(this.body);
          if (!root.isConnected) return;
          clearTimeout(this.timers.get(root));
          button.replaceChildren(createIconElement(Check));
          status.textContent = t('plugins.codeStyler.copied');
          this.timers.set(root, setTimeout(() => {
            button.replaceChildren(createIconElement(Copy));
            status.textContent = '';
            this.timers.delete(root);
          }, 1500));
        } catch {
          status.textContent = t('plugins.codeStyler.copyFailed');
        }
      });
      root.append(button, status);
    }
    return root;
  }
  destroy(dom: HTMLElement) { clearTimeout(this.timers.get(dom)); }
}

export function codeStylerExtension(settings: PluginSettings['codeStyler']): Extension {
  const blockCache = new WeakMap<EditorView['state']['doc'], ReturnType<typeof codeBlocks>>();
  const blocksFor = (state: EditorView['state']) => {
    const cached = blockCache.get(state.doc);
    if (cached) return cached;
    // Неполное инкрементальное дерево ещё хранит старые границы после вставки.
    const tree = syntaxTreeAvailable(state, state.doc.length)
      ? syntaxTree(state) : editorMarkdownSupport.language.parser.parse(state.doc.toString());
    const blocks = codeBlocks(tree, state.doc, [{ from: 0, to: state.doc.length }]);
    blockCache.set(state.doc, blocks);
    return blocks;
  };
  const build = (state: EditorView['state']) => {
    const decorations = [];
    const visibleBlocks: CodeBlockShape[] = [];
    for (const block of blocksFor(state)) {
      const first = state.doc.lineAt(block.from);
      const fence = /^ {0,3}(`{3,}|~{3,})/.exec(first.text);
      if (!fence) { visibleBlocks.push(block); continue; }
      const info = parseInfo(block.info);
      const last = state.doc.lineAt(block.end);
      const closing = last.number > first.number && new RegExp(`^ {0,3}${fence[1][0]}{${fence[1].length},}\\s*$`).test(last.text);
      const bodyFrom = Math.min(first.to + 1, state.doc.length);
      const bodyTo = closing ? last.from : block.end;
      const body = state.doc.sliceString(bodyFrom, Math.max(bodyFrom, bodyTo));
      const hidden = previewCaret(state) < block.from || previewCaret(state) > block.end;
      const bodyLast = last.number - (closing ? 1 : 0);
      const hasBody = bodyLast > first.number;
      const visibleFirst = hidden && hasBody ? state.doc.line(first.number + 1).from : first.from;
      if (hidden) {
        // Inclusive start не создаёт пустую строку; открытый конец сохраняет floating copy на теле.
        decorations.push(Decoration.replace({ block: true, inclusiveStart: true, inclusiveEnd: false }).range(first.from, bodyFrom));
        if (closing) decorations.push(Decoration.replace({ block: true, inclusiveStart: true, inclusiveEnd: last.to === state.doc.length }).range(last.from, Math.min(last.to + 1, state.doc.length)));
        if (hasBody) visibleBlocks.push({ ...block, from: visibleFirst, end: state.doc.line(bodyLast).to });
      } else {
        visibleBlocks.push(block);
      }
      const caption = settings.header ? info.title ?? info.lang : '';
      if (caption || settings.copyButton) {
        decorations.push(Decoration.widget({ widget: new CodeHeader(caption, body, settings.copyButton), block: Boolean(caption) || (hidden && !hasBody), side: caption || (hidden && !hasBody) ? -1 : 1 }).range(caption ? first.from : visibleFirst));
        if (caption) decorations.push(Decoration.line({ class: 'q-code-styler-captioned' }).range(visibleFirst));
      }
      const start = typeof info.ln === 'number' ? info.ln : 1;
      const numbered = info.ln === undefined ? settings.lineNumbers : info.ln !== false;
      const width = `${String(start + Math.max(0, bodyLast - first.number - 1)).length}ch`;
      for (let number = first.number + 1; number <= bodyLast; number++) {
        const relative = number - first.number;
        const classes = [numbered ? 'q-md-code-numbered' : '', info.hl.has(relative) ? 'q-md-code-line--hl' : ''].filter(Boolean).join(' ');
        if (classes) decorations.push(Decoration.line({ attributes: { class: classes, ...(numbered ? { 'data-ln': String(start + relative - 1), style: `--q-code-ln-width: ${width}` } : {}) } }).range(state.doc.line(number).from));
      }
    }
    const lines = codeBlockDecorations(state.doc, visibleBlocks);
    for (const iter = lines.iter(); iter.value; iter.next()) {
      decorations.push(iter.value.range(iter.from, iter.to));
    }
    return Decoration.set(decorations, true);
  };
  const field = StateField.define<DecorationSet>({
    create: build,
    update(value, transaction) {
      if (transaction.docChanged || transaction.selection
        || transaction.state.facet(revealAtCaretFacet) !== transaction.startState.facet(revealAtCaretFacet)
        || syntaxTree(transaction.startState) !== syntaxTree(transaction.state)) return build(transaction.state);
      return value;
    },
    provide: field => EditorView.decorations.from(field),
  });
  const hover = (event: MouseEvent, view: EditorView) => {
    const line = (event.target as HTMLElement)?.closest?.('.q-md-code-line');
    const position = line ? view.posAtDOM(line) : -1;
    const block = blocksFor(view.state)
      .find(block => position >= block.from && position <= block.end);
    for (const root of view.dom.querySelectorAll<HTMLElement>('.q-code-styler-floating')) {
      root.classList.toggle('is-hovered', !!block && !!root.parentElement && view.posAtDOM(root.parentElement) >= block.from && view.posAtDOM(root.parentElement) <= block.end);
    }
    return false;
  };
  return [codeBlockLineDecorationsFacet.of(false), field, EditorView.domEventHandlers({
    mouseover: hover,
    mouseout(event, view) {
      if (!view.dom.contains(event.relatedTarget as Node | null)) {
        for (const root of view.dom.querySelectorAll('.q-code-styler-floating')) root.classList.remove('is-hovered');
      }
      return false;
    },
  }), EditorView.baseTheme({
    '.q-code-styler-header': { display: 'flex', alignItems: 'center', gap: 'var(--q-gap-xs)', padding: 'var(--q-padding-4xs) var(--q-block-padding)', color: 'var(--q-text-secondary)', background: 'var(--q-block-bg)', border: 'var(--q-border-1) solid var(--q-block-border)', borderBottom: '0', borderTopLeftRadius: 'var(--q-block-radius)', borderTopRightRadius: 'var(--q-block-radius)', marginBottom: '0' },
    '.q-md-code-copy': { marginLeft: 'auto' },
    '.cm-line.q-md-code-line--first.q-code-styler-captioned': { paddingTop: '0', marginTop: '0' },
    '.cm-line.q-md-code-line--first.q-code-styler-captioned::before': { borderTop: '0', borderTopLeftRadius: '0', borderTopRightRadius: '0' },
    '.q-code-styler-floating': { position: 'absolute', right: 'var(--q-block-padding)', top: 'var(--q-padding-4xs)', zIndex: '1', opacity: '0', pointerEvents: 'none' },
    '.q-code-styler-floating:focus-within, .q-code-styler-floating.is-hovered': { opacity: '1', pointerEvents: 'auto' },
    '.q-code-styler-status': { position: 'absolute', width: '1px', height: '1px', padding: '0', margin: '-1px', overflow: 'hidden', clipPath: 'inset(50%)', whiteSpace: 'nowrap', border: '0' },
    '.cm-line.q-md-code-line.q-md-code-numbered': { paddingLeft: 'calc(var(--q-block-padding) + var(--q-code-ln-width) + 1ch)', position: 'relative' },
    '.cm-line.q-md-code-line.q-md-code-numbered::after': { content: 'attr(data-ln)', position: 'absolute', left: 'var(--q-block-padding)', width: 'var(--q-code-ln-width)', textAlign: 'right', color: 'var(--q-text-secondary)', pointerEvents: 'none' },
    '.cm-line.q-md-code-line.q-md-code-line--hl::before': { background: 'color-mix(in srgb, var(--q-bg-accent) 20%, var(--q-block-bg))' },
  })];
}
