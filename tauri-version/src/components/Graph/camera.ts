import { clamp } from '../../modules/math';
import type { GraphBounds } from '../../modules/graph';
import { approachStep } from './easing';

const MIN_SCALE = 0.02;
const MAX_SCALE = 400;
const RESPONSE_SECONDS = 0.08;
const SETTLED_ZOOM = 0.0015;
const SETTLED_PIXELS = 0.05;

interface ZoomAnchor {
  offsetX: number;
  offsetY: number;
  worldX: number;
  worldY: number;
}

interface CenterTarget {
  x: number;
  y: number;
}

export class Camera {
  centerX = 0;
  centerY = 0;
  scale = 1;
  private targetScale = 1;
  private anchor: ZoomAnchor | null = null;
  private target: CenterTarget | null = null;

  zoomBy(factor: number, offsetX: number, offsetY: number): void {
    const world = this.toWorld(offsetX, offsetY);
    this.anchor = { offsetX, offsetY, worldX: world.x, worldY: world.y };
    this.target = null;
    this.targetScale = clamp(this.targetScale * factor, MIN_SCALE, MAX_SCALE);
  }

  panBy(dx: number, dy: number): void {
    this.target = null;
    const worldDx = dx / this.scale;
    const worldDy = dy / this.scale;
    this.centerX -= worldDx;
    this.centerY += worldDy;
    if (this.anchor) {
      this.anchor.worldX -= worldDx;
      this.anchor.worldY += worldDy;
    }
  }

  rescaleWorld(factor: number): void {
    this.centerX *= factor;
    this.centerY *= factor;
    if (this.anchor) {
      this.anchor.worldX *= factor;
      this.anchor.worldY *= factor;
    }
    if (this.target) {
      this.target.x *= factor;
      this.target.y *= factor;
    }
  }

  moveTo(centerX: number, centerY: number, scale: number): void {
    this.centerX = centerX;
    this.centerY = centerY;
    this.scale = clamp(scale, MIN_SCALE, MAX_SCALE);
    this.targetScale = this.scale;
    this.anchor = null;
    this.target = null;
  }

  jumpTo(bounds: GraphBounds, width: number, height: number): void {
    const view = fitView(bounds, width, height);
    this.centerX = view.x;
    this.centerY = view.y;
    this.scale = view.scale;
    this.targetScale = view.scale;
    this.anchor = null;
    this.target = null;
  }

  glideTo(bounds: GraphBounds, width: number, height: number): void {
    const view = fitView(bounds, width, height);
    this.targetScale = view.scale;
    this.target = { x: view.x, y: view.y };
    this.anchor = null;
  }

  advance(seconds: number): boolean {
    const step = approachStep(seconds, RESPONSE_SECONDS);
    const zooming = this.advanceScale(step);
    const gliding = this.advanceCenter(step);
    return zooming || gliding;
  }

  toWorld(offsetX: number, offsetY: number): { x: number; y: number } {
    return {
      x: this.centerX + offsetX / this.scale,
      y: this.centerY - offsetY / this.scale,
    };
  }

  private advanceScale(step: number): boolean {
    const gap = this.targetScale / this.scale;
    if (Math.abs(Math.log(gap)) < SETTLED_ZOOM) {
      const settled = this.scale !== this.targetScale;
      this.scale = this.targetScale;
      this.holdAnchor();
      this.anchor = null;
      return settled;
    }
    this.scale *= gap ** step;
    this.holdAnchor();
    return true;
  }

  private advanceCenter(step: number): boolean {
    const target = this.target;
    if (!target) return false;
    const dx = target.x - this.centerX;
    const dy = target.y - this.centerY;
    if (Math.hypot(dx, dy) * this.scale < SETTLED_PIXELS) {
      this.centerX = target.x;
      this.centerY = target.y;
      this.target = null;
      return false;
    }
    this.centerX += dx * step;
    this.centerY += dy * step;
    return true;
  }

  private holdAnchor(): void {
    if (!this.anchor) return;
    this.centerX = this.anchor.worldX - this.anchor.offsetX / this.scale;
    this.centerY = this.anchor.worldY + this.anchor.offsetY / this.scale;
  }
}

function fitView(bounds: GraphBounds, width: number, height: number) {
  return {
    x: (bounds.minX + bounds.maxX) / 2,
    y: (bounds.minY + bounds.maxY) / 2,
    scale: clamp(
      Math.min(
        width / Math.max(bounds.maxX - bounds.minX, 1e-3),
        height / Math.max(bounds.maxY - bounds.minY, 1e-3),
      ),
      MIN_SCALE,
      MAX_SCALE,
    ),
  };
}
