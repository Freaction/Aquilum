import { t } from '../../i18n';

const NOTICEABLE_STEP = 0.005;
const QUIET_MS = 100;

export class ScaleReadout {
  private element: HTMLElement | null = null;
  private painted = -1;
  private paintedAt = Number.NEGATIVE_INFINITY;

  attach(element: HTMLElement | null): void {
    this.element = element;
    this.painted = -1;
    this.paintedAt = Number.NEGATIVE_INFINITY;
  }

  paint(scale: number, time: number, continues: boolean): void {
    const element = this.element;
    if (!element) return;
    if (Math.abs(scale - this.painted) <= this.painted * NOTICEABLE_STEP) return;
    if (continues && time - this.paintedAt < QUIET_MS) return;
    this.painted = scale;
    this.paintedAt = time;
    element.textContent = t('graph.scale', { scale: scale.toFixed(2) });
  }
}
