import { ViewPlugin, type EditorView } from '@codemirror/view';

const BLOCK_WIDTH_VARIABLE = '--q-editor-block-width';

class BlockWidth {
  private readonly observer: ResizeObserver;
  private applied = 0;

  constructor(private readonly view: EditorView) {
    this.observer = new ResizeObserver(() => this.sync());
    this.observer.observe(view.scrollDOM);
    const column = view.dom.closest('.q-editor-content');
    if (column) this.observer.observe(column);
    this.sync();
  }

  destroy(): void {
    this.observer.disconnect();
  }

  private sync(): void {
    const style = getComputedStyle(this.view.contentDOM);
    const padding = parseFloat(style.paddingLeft || '0') + parseFloat(style.paddingRight || '0');
    const width = Math.floor(this.view.scrollDOM.clientWidth - padding);
    if (width <= 0 || width === this.applied) return;
    this.applied = width;
    this.view.dom.style.setProperty(BLOCK_WIDTH_VARIABLE, `${width}px`);
  }
}

export function blockWidthExtension() {
  return ViewPlugin.fromClass(BlockWidth);
}
