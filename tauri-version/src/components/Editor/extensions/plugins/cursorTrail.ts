import { EditorView, ViewPlugin, type ViewUpdate } from '@codemirror/view';
import './cursorTrail.css';

export const cursorTrailExtension = ViewPlugin.fromClass(class {
  private media = window.matchMedia('(prefers-reduced-motion: reduce)');
  private layer: HTMLElement | null = null;
  private timer: ReturnType<typeof setTimeout> | undefined;
  private alive = true;
  private clear = () => {
    clearTimeout(this.timer);
    this.layer?.remove();
    this.layer = null;
  };

  constructor(private view: EditorView) {
    this.media.addEventListener('change', this.clear);
  }

  update(update: ViewUpdate) {
    if (!update.selectionSet || this.media.matches) return;
    const previous = update.startState.selection.main.head;
    const from = update.changes.mapPos(previous, -1);
    const to = update.state.selection.main.head;
    if (Math.abs(to - previous) <= 1) return;
    this.view.requestMeasure({
      key: this,
      read: view => {
        if (!this.alive || this.media.matches || view.state.doc !== update.state.doc
          || view.state.selection.main.head !== to) return null;
        const start = view.coordsAtPos(from);
        const end = view.coordsAtPos(to);
        const rect = view.dom.getBoundingClientRect();
        return start && end ? {
          left: start.left - rect.left,
          top: (start.top + start.bottom) / 2 - rect.top,
          dx: end.left - start.left,
          dy: (end.top + end.bottom - start.top - start.bottom) / 2,
        } : null;
      },
      write: coords => {
        if (!this.alive || this.media.matches || !coords) return;
        this.clear();
        const layer = document.createElement('div');
        layer.className = 'q-cursor-trail';
        layer.setAttribute('aria-hidden', 'true');
        layer.style.left = `${coords.left}px`;
        layer.style.top = `${coords.top}px`;
        layer.style.width = `${Math.hypot(coords.dx, coords.dy)}px`;
        layer.style.transform = `rotate(${Math.atan2(coords.dy, coords.dx)}rad)`;
        this.view.dom.appendChild(layer);
        this.layer = layer;
        this.timer = setTimeout(this.clear, 150);
      },
    });
  }

  destroy() {
    this.alive = false;
    this.media.removeEventListener('change', this.clear);
    this.clear();
  }
});
