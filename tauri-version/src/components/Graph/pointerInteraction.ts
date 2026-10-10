import { attachPointerControls } from './pointerControls';

interface InteractionPort {
  viewportWidth(): number;
  viewportHeight(): number;
  panBy(dx: number, dy: number): void;
  zoomBy(factor: number, centeredX: number, centeredY: number): void;
  nodeAt(centeredX: number, centeredY: number): number;
  hoverSettled(): boolean;
  hoverChanged(node: number): void;
  selected(node: number): void;
  reset(): void;
  invalidate(): void;
}

export class PointerInteraction {
  private readonly release: () => void;
  private paddingBoxX = 0;
  private paddingBoxY = 0;
  private inside = false;
  private panning = false;
  private hovered = -1;

  constructor(
    private readonly canvas: HTMLCanvasElement,
    private readonly port: InteractionPort,
  ) {
    this.release = attachPointerControls(canvas, {
      panBy: (dx, dy) => {
        port.panBy(dx, dy);
        port.invalidate();
      },
      dragging: (active) => {
        this.panning = active;
        this.applyCursor();
      },
      zoomBy: (factor, paddingBoxX, paddingBoxY) => {
        this.aimAt(paddingBoxX, paddingBoxY);
        port.zoomBy(factor, this.centeredX(paddingBoxX), this.centeredY(paddingBoxY));
        port.invalidate();
      },
      pointerAt: (paddingBoxX, paddingBoxY) => {
        this.aimAt(paddingBoxX, paddingBoxY);
        this.resolveHover();
      },
      select: (paddingBoxX, paddingBoxY) => {
        const node = port.nodeAt(this.centeredX(paddingBoxX), this.centeredY(paddingBoxY));
        if (node >= 0) port.selected(node);
      },
      leave: () => {
        this.inside = false;
        this.resolveHover();
      },
      reset: () => port.reset(),
    });
    this.applyCursor();
  }

  get node(): number {
    return this.hovered;
  }

  forget(): void {
    this.hovered = -1;
    this.applyCursor();
  }

  resolveHover(): void {
    if (!this.port.hoverSettled()) return;
    const wanted = this.inside && !this.panning
      ? this.port.nodeAt(
        this.centeredX(this.paddingBoxX),
        this.centeredY(this.paddingBoxY),
      )
      : -1;
    if (wanted === this.hovered) return;
    this.hovered = wanted;
    this.port.hoverChanged(wanted);
    this.applyCursor();
  }

  detach(): void {
    this.release();
  }

  private aimAt(paddingBoxX: number, paddingBoxY: number): void {
    this.paddingBoxX = paddingBoxX;
    this.paddingBoxY = paddingBoxY;
    this.inside = true;
  }

  private centeredX(paddingBoxX: number): number {
    return paddingBoxX - this.port.viewportWidth() / 2;
  }

  private centeredY(paddingBoxY: number): number {
    return paddingBoxY - this.port.viewportHeight() / 2;
  }

  private applyCursor(): void {
    this.canvas.style.cursor = this.panning
      ? 'grabbing'
      : this.hovered >= 0
        ? 'pointer'
        : 'default';
  }
}
