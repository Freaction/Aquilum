import { EditorView } from '@codemirror/view';

export function separatePastedFrontmatter(text: string): string {
  const eol = text.includes('\r\n') ? '\r\n' : '\n';
  return text.replace(
    /^(---\r?\n[\s\S]*?\r?\n---)(?:\r?\n(?!\r?\n)|$)/,
    `$1${eol}${eol}`,
  );
}

export const frontmatterPaste = EditorView.clipboardInputFilter.of(separatePastedFrontmatter);
