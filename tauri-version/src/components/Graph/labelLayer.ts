import { clamp } from '../../modules/math';
import { CandidatePicker, type Candidate, type LabelScene } from './labelCandidates';
import type { LabelSprites } from './labelSprites';
import type { NoteLabels } from './noteLabels';
import { approachStep, FADE_SECONDS, SETTLED_FADE } from './easing';

const READABLE_SCALE = 50;
const FULLY_READABLE_SCALE = 100;
const NODE_GAP_PIXELS = 4;

export interface LabelTypography {
  font: string;
  lineHeight: number;
}

export class LabelLayer {
  private readonly fades = new Map<number, number>();
  private readonly picker = new CandidatePicker();
  private readonly view = { width: 0, height: 0 };
  private width = 0;
  private height = 0;
  private ratio = 1;

  constructor(
    private readonly sprites: LabelSprites,
    private readonly labels: NoteLabels,
  ) {}

  restyle(typography: LabelTypography, ratio: number): void {
    this.ratio = ratio;
    this.sprites.atlas.configure(typography.font, ratio, typography.lineHeight);
    this.sprites.reserve(this.sprites.atlas.capacity);
    this.picker.setLimit(this.sprites.atlas.capacity);
  }

  resize(width: number, height: number): void {
    this.width = width;
    this.height = height;
  }

  forget(): void {
    this.fades.clear();
    this.sprites.atlas.forget();
  }

  clear(): void {
    this.fades.clear();
    this.sprites.begin();
  }

  draw(scene: LabelScene, seconds: number, dimOf: (node: number) => number): boolean {
    this.sprites.begin();
    const readable = readableAt(scene.scale * scene.spread);
    if (readable === 0 && scene.hovered < 0 && this.fades.size === 0) return false;
    if (!scene.grid || scene.nodes.nodeCount === 0) {
      this.fades.clear();
      return false;
    }
    this.view.width = this.width;
    this.view.height = this.height;
    const entries = this.picker.collect(scene, this.view, this.fades, readable);
    const step = approachStep(seconds, FADE_SECONDS);
    let animating = false;
    for (const entry of entries) {
      if (this.paint(entry, scene, readable, step, dimOf)) animating = true;
    }
    return animating;
  }

  private paint(
    entry: Candidate,
    scene: LabelScene,
    readable: number,
    step: number,
    dimOf: (node: number) => number,
  ): boolean {
    const label = this.labels.get(entry.node);
    if (!label) {
      this.labels.request(entry.node);
      return this.fadeOut(entry, step);
    }
    const centerX = (scene.nodes.x(entry.node) * scene.spread - scene.centerX)
      * scene.scale
      + this.width / 2;
    const top = (scene.centerY - scene.nodes.y(entry.node) * scene.spread)
      * scene.scale
      + this.height / 2
      + entry.radius
      + NODE_GAP_PIXELS;
    const onScreen = centerX > -this.width && centerX < this.width * 2
      && top > -this.height && top < this.height;
    const target = onScreen && entry.eligible ? 1 : 0;
    const alpha = this.advance(entry, target, step);
    const animating = alpha !== target;
    if (alpha <= SETTLED_FADE || !onScreen) return animating;
    const focused = entry.node === scene.hovered;
    const cell = this.sprites.atlas.slot(this.sprites.atlas.fit(label.title));
    if (!cell) return animating;
    this.sprites.place(
      cell,
      centerX,
      top,
      this.ratio,
      alpha * (focused ? 1 : readable) * dimOf(entry.node),
      focused,
    );
    return animating;
  }

  private advance(entry: Candidate, target: number, step: number): number {
    const alpha = entry.fade + (target - entry.fade) * step;
    if (alpha <= SETTLED_FADE && target === 0) {
      this.fades.delete(entry.node);
      return 0;
    }
    const settled = alpha >= 1 - SETTLED_FADE && target === 1 ? 1 : alpha;
    this.fades.set(entry.node, settled);
    return settled;
  }

  private fadeOut(entry: Candidate, step: number): boolean {
    if (entry.fade <= SETTLED_FADE) {
      this.fades.delete(entry.node);
      return false;
    }
    this.advance(entry, 0, step);
    return true;
  }
}

export function readableAt(scale: number): number {
  return clamp(
    (scale - READABLE_SCALE) / (FULLY_READABLE_SCALE - READABLE_SCALE),
    0,
    1,
  );
}
