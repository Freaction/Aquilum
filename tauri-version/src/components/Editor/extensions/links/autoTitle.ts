import type { Extension } from '@codemirror/state';
import { EditorView } from '@codemirror/view';
import {
  escapeMarkdownTitle,
  fetchPageTitle,
  isBareHttpUrl,
  isImageUrl,
  isMarkdownLinkContext,
  makeTitlePlaceholder,
  markdownLink,
} from '../../../../modules/linkTitle';

export function insertClipboardAsLink(view: EditorView, clipboardText: string): boolean {
  const text = clipboardText.trim();
  if (!isBareHttpUrl(text) || isImageUrl(text)) return false;

  const { from, to } = view.state.selection.main;
  const before = view.state.sliceDoc(Math.max(0, from - 64), from);
  if (isMarkdownLinkContext(before)) return false;

  const selected = view.state.sliceDoc(from, to).trim();
  if (selected) {
    view.dispatch(view.state.replaceSelection(markdownLink(escapeMarkdownTitle(selected), text)));
    return true;
  }

  const placeholder = makeTitlePlaceholder();
  view.dispatch(view.state.replaceSelection(markdownLink(placeholder, text)));
  void replacePlaceholderTitle(view, placeholder, text);
  return true;
}

async function replacePlaceholderTitle(
  view: EditorView,
  placeholder: string,
  url: string,
): Promise<void> {
  const title = escapeMarkdownTitle(await fetchPageTitle(url));
  if (!view.dom.isConnected) return;
  const start = view.state.doc.toString().indexOf(placeholder);
  if (start < 0) return;
  view.dispatch({
    changes: { from: start, to: start + placeholder.length, insert: title },
  });
}

export function autoLinkTitleExtension(enabled: boolean): Extension {
  if (!enabled) return [];
  return EditorView.domEventHandlers({
    paste(event, view) {
      const text = event.clipboardData?.getData('text/plain');
      if (!text || !insertClipboardAsLink(view, text)) return false;
      event.preventDefault();
      return true;
    },
    drop(event, view) {
      const text = event.dataTransfer?.getData('text/plain');
      if (!text || !insertClipboardAsLink(view, text)) return false;
      event.preventDefault();
      return true;
    },
  });
}
