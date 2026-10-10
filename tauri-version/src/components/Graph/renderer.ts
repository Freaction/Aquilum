import type { GraphSnapshot } from '../../modules/graph';
import type { GraphCameraState } from '../../modules/ui-state';
import { Camera } from './camera';
import { CanvasSizer } from './canvasSize';
import { DateFilter } from './dateFilter';
import { FrameClock, type FrameStep } from './frameClock';
import { DEFAULT_DISPLAY, type GraphDisplay } from './graphDisplay';
import { GpuScene } from './gpuScene';
import { HighlightMap } from './highlightMap';
import { LabelLayer, type LabelTypography } from './labelLayer';
import type { NoteLabels } from './noteLabels';
import { dimmedBy, MAX_EDGES_PER_FRAME } from './nodeMetrics';
import { readPalette, type Palette } from './palette';
import { pickNode } from './pickGrid';
import { PointerInteraction } from './pointerInteraction';
import { ScaleReadout } from './scaleReadout';
import { SnapshotStore } from './snapshotStore';

export type { GraphDateRange } from './snapshotStore';

const CAMERA_SETTLE_MS = 400;

export class GraphRenderer {
  private readonly scene: GpuScene;
  private readonly store = new SnapshotStore();
  private readonly highlight = new HighlightMap();
  private readonly filter = new DateFilter();
  private readonly camera = new Camera();
  private readonly labelLayer: LabelLayer;
  private readonly readout = new ScaleReadout();
  private readonly clock = new FrameClock();
  private readonly sizer: CanvasSizer;
  private readonly interaction: PointerInteraction;
  private readonly themeWatcher: MutationObserver;
  private readonly handleContextLost: (event: Event) => void;
  private palette: Palette;
  private display: GraphDisplay = DEFAULT_DISPLAY;
  private labelsAnimating = false;
  private highlightAnimating = false;
  private visibleNodes = 0;
  private ratio = 1;
  private cssWidth = 1;
  private cssHeight = 1;
  private cameraSaveTimer = 0;

  onSelect: ((node: number) => void) | null = null;
  onCameraSettled: ((camera: GraphCameraState) => void) | null = null;
  onVisibleNodes: ((count: number) => void) | null = null;
  onContextLost: (() => void) | null = null;

  constructor(
    private readonly canvas: HTMLCanvasElement,
    labels: NoteLabels,
  ) {
    this.scene = new GpuScene(canvas);
    this.palette = readPalette(canvas);
    this.labelLayer = new LabelLayer(this.scene.labels, labels);
    this.sizer = new CanvasSizer(canvas, () => this.resize());
    this.interaction = new PointerInteraction(canvas, {
      viewportWidth: () => this.cssWidth,
      viewportHeight: () => this.cssHeight,
      panBy: (dx, dy) => {
        this.camera.panBy(dx, dy);
        this.markCameraMoved();
      },
      zoomBy: (factor, centeredX, centeredY) => {
        this.camera.zoomBy(factor, centeredX, centeredY);
        this.markCameraMoved();
      },
      nodeAt: (centeredX, centeredY) => this.nodeAt(centeredX, centeredY),
      hoverSettled: () => !this.store.isMorphing(),
      hoverChanged: () => this.relight(),
      selected: (node) => this.onSelect?.(node),
      reset: () => this.fit(),
      invalidate: () => this.invalidate(),
    });
    this.handleContextLost = (event) => {
      event.preventDefault();
      this.clock.abandon();
      this.onContextLost?.();
    };
    canvas.addEventListener('webglcontextlost', this.handleContextLost);
    this.themeWatcher = new MutationObserver(() => this.refreshPalette());
    this.themeWatcher.observe(document.documentElement, { attributeFilter: ['data-theme'] });
  }

  setSnapshot(snapshot: GraphSnapshot, keepCamera = false): void {
    const settled = this.store.snapshot !== null;
    const morphs = this.store.morphsInto(snapshot, keepCamera);
    this.store.adopt(snapshot, morphs);
    this.highlight.resize(this.store.rows);
    this.interaction.forget();
    this.labelLayer.forget();
    this.scene.textures.uploadNodes(this.store.texels, this.store.rows);
    this.scene.textures.uploadFreshness(this.store.freshnessTexture(), this.store.rows);
    this.scene.textures.allocateHighlight(this.highlight.data, this.store.rows);
    this.filter.adopt(
      snapshot.createdDays,
      snapshot.nodeCount,
      this.store.rows,
      this.display.createdFrom,
    );
    this.scene.textures.allocateReveal(this.filter.data, this.store.rows);
    this.scene.uploadEdges(snapshot.edges);
    this.reportVisible();
    if (keepCamera && settled) this.invalidate();
    else this.jumpToFit();
  }

  createdRange() {
    return this.store.createdRange();
  }

  applyDisplay(next: GraphDisplay): void {
    const current = this.display;
    if (matches(current, next)) return;
    this.display = next;
    if (next.spread !== current.spread) this.camera.rescaleWorld(next.spread / current.spread);
    if (next.createdFrom !== current.createdFrom) this.refilter();
    if (next.highlightDepth !== current.highlightDepth) this.relight();
    if (current.labels && !next.labels) this.labelLayer.clear();
    this.invalidate();
  }

  setScaleReadout(element: HTMLElement | null): void {
    this.readout.attach(element);
    this.readout.paint(this.camera.scale, this.clock.time, false);
  }

  redraw(): void {
    this.invalidate();
  }

  fit(): void {
    if (!this.store.snapshot) return;
    this.camera.glideTo(this.store.bounds(this.display.spread), this.cssWidth, this.cssHeight);
    this.markCameraMoved();
    this.invalidate();
  }

  cameraState(): GraphCameraState {
    return {
      centerX: this.camera.centerX / this.display.spread,
      centerY: this.camera.centerY / this.display.spread,
      scale: this.camera.scale,
    };
  }

  applyCamera(camera: GraphCameraState): void {
    this.camera.moveTo(
      camera.centerX * this.display.spread,
      camera.centerY * this.display.spread,
      camera.scale,
    );
    this.invalidate();
  }

  hasSnapshot(): boolean {
    return this.store.snapshot !== null;
  }

  resize(): void {
    const { metrics, resized } = this.sizer.read();
    this.cssWidth = metrics.cssWidth;
    this.cssHeight = metrics.cssHeight;
    if (!resized) return;
    this.ratio = metrics.ratio;
    this.scene.setViewport(metrics.deviceWidth, metrics.deviceHeight, metrics.ratio);
    this.labelLayer.resize(metrics.cssWidth, metrics.cssHeight);
    this.labelLayer.restyle(this.typography(), metrics.ratio);
    this.draw(0);
    if (this.labelsAnimating) this.invalidate();
  }

  refreshPalette(): void {
    this.palette = readPalette(this.canvas);
    this.labelLayer.restyle(this.typography(), this.ratio);
    this.invalidate();
  }

  private typography(): LabelTypography {
    const styles = getComputedStyle(this.canvas);
    const size = Number.parseFloat(styles.fontSize) || 12;
    const declared = Number.parseFloat(styles.lineHeight);
    return {
      font: `${styles.fontWeight} ${styles.fontSize} ${styles.fontFamily}`,
      lineHeight: Number.isFinite(declared) ? declared : Math.round(size * 1.35),
    };
  }

  dispose(): void {
    this.saveCameraNow();
    this.clock.stop();
    this.themeWatcher.disconnect();
    this.interaction.detach();
    this.canvas.removeEventListener('webglcontextlost', this.handleContextLost);
    this.sizer.dispose();
    this.scene.dispose();
  }

  private markCameraMoved(): void {
    if (this.cameraSaveTimer) window.clearTimeout(this.cameraSaveTimer);
    this.cameraSaveTimer = window.setTimeout(() => {
      this.cameraSaveTimer = 0;
      this.onCameraSettled?.(this.cameraState());
    }, CAMERA_SETTLE_MS);
  }

  private saveCameraNow(): void {
    window.clearTimeout(this.cameraSaveTimer);
    this.cameraSaveTimer = 0;
    if (this.store.snapshot) this.onCameraSettled?.(this.cameraState());
  }

  private jumpToFit(): void {
    if (!this.store.snapshot) return;
    this.camera.jumpTo(this.store.bounds(this.display.spread), this.cssWidth, this.cssHeight);
    this.invalidate();
  }

  private nodeAt(centeredX: number, centeredY: number): number {
    const grid = this.store.grid;
    const snapshot = this.store.snapshot;
    if (!snapshot || !grid) return -1;
    const world = this.camera.toWorld(centeredX, centeredY);
    const node = pickNode(
      grid,
      snapshot,
      world.x,
      world.y,
      this.camera.scale,
      this.display.nodeSize,
      this.display.spread,
    );
    if (node < 0) return -1;
    return this.store.createdDay(node) < this.display.createdFrom ? -1 : node;
  }

  private relight(): void {
    this.highlight.lightUp(this.store.adjacency, this.interaction.node, this.display.highlightDepth);
    this.highlightAnimating = true;
    this.invalidate();
  }

  private uploadHighlight(): void {
    const { firstRow, rowCount } = this.highlight.dirtyRows();
    this.scene.textures.uploadHighlightRows(this.highlight.data, firstRow, rowCount);
    this.highlight.settle();
  }

  private uploadReveal(): void {
    const { firstRow, rowCount } = this.filter.dirtyRows();
    this.scene.textures.uploadRevealRows(this.filter.data, firstRow, rowCount);
    this.filter.settle();
  }

  private refilter(): void {
    const snapshot = this.store.snapshot;
    if (!snapshot) return;
    this.filter.retarget(snapshot.createdDays, snapshot.nodeCount, this.display.createdFrom);
    this.reportVisible();
  }

  private reportVisible(): void {
    if (this.filter.visible === this.visibleNodes) return;
    this.visibleNodes = this.filter.visible;
    this.onVisibleNodes?.(this.visibleNodes);
  }

  private invalidate(): void {
    this.clock.request(this.step);
  }

  private readonly step: FrameStep = (seconds, time) => {
    const zooming = this.camera.advance(seconds);
    if (this.store.advanceMorph(seconds)) {
      this.scene.textures.uploadNodes(this.store.texels, this.store.rows);
    }
    const morphing = this.store.isMorphing();
    this.interaction.resolveHover();
    this.highlightAnimating = this.highlight.advance(seconds);
    this.uploadHighlight();
    const revealing = this.filter.advance(seconds);
    this.uploadReveal();
    this.draw(seconds);
    const continues = zooming
      || morphing
      || revealing
      || this.labelsAnimating
      || this.highlightAnimating;
    this.readout.paint(this.camera.scale, time, continues);
    return continues;
  };

  private draw(seconds: number): void {
    this.scene.beginFrame();
    if (this.store.nodeCount === 0) {
      this.labelsAnimating = false;
      this.highlightAnimating = false;
      return;
    }
    const display = this.display;
    const hovered = this.interaction.node;
    const dimming = this.highlight.dimming;
    const edgeCount = this.store.edgeCount;
    const view = {
      centerX: this.camera.centerX,
      centerY: this.camera.centerY,
      deviceScale: this.camera.scale * this.ratio,
      spread: display.spread,
    };
    this.scene.drawEdges(view, {
      drawnEdges: Math.min(edgeCount, MAX_EDGES_PER_FRAME),
      edgeCount,
      dimming,
    }, this.palette);
    const heat = display.heatmapAxis === 'created' ? this.store.created : this.store.freshness;
    this.scene.drawNodes(view, {
      nodeCount: this.store.nodeCount,
      sizeScale: display.nodeSize,
      dimming,
      heatmap: display.heatmapAxis !== 'none',
      heatCreated: display.heatmapAxis === 'created',
      oldest: heat.oldest,
      newest: heat.newest,
    }, this.palette);
    this.labelsAnimating = display.labels && this.labelLayer.draw({
      nodes: this.store,
      grid: this.store.grid,
      centerX: this.camera.centerX,
      centerY: this.camera.centerY,
      scale: this.camera.scale,
      sizeScale: display.nodeSize,
      spread: display.spread,
      createdFrom: display.createdFrom,
      hovered,
    }, seconds, (node) => dimmedBy(this.highlight.stateOf(node), dimming));
    if (display.labels) this.scene.drawLabels(this.palette);
  }
}

function matches(current: GraphDisplay, next: GraphDisplay): boolean {
  return current.nodeSize === next.nodeSize
    && current.spread === next.spread
    && current.highlightDepth === next.highlightDepth
    && current.labels === next.labels
    && current.heatmapAxis === next.heatmapAxis
    && current.createdFrom === next.createdFrom;
}
