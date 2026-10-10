import { t } from '../../i18n';
import {
  bindCorner,
  bindEdgeEndpoints,
  createBuffer,
  createProgram,
  createVao,
  UniformCache,
} from './glResources';
import {
  EDGE_MAX_HALF_PIXELS,
  EDGE_MIN_HALF_PIXELS,
  MAX_NODE_PIXELS,
  SUB_PIXEL_RADIUS,
} from './nodeMetrics';
import { LabelSprites } from './labelSprites';
import {
  FRESHNESS_UNIT,
  HIGHLIGHT_UNIT,
  NODE_UNIT,
  NodeTextures,
  REVEAL_UNIT,
} from './nodeTextures';
import { fadeEdges, type Palette } from './palette';
import { EDGE_FRAGMENT, EDGE_VERTEX, NODE_FRAGMENT, NODE_VERTEX, QUAD } from './shaders';

interface CameraView {
  centerX: number;
  centerY: number;
  deviceScale: number;
  spread: number;
}

interface NodePass {
  nodeCount: number;
  sizeScale: number;
  dimming: number;
  heatmap: boolean;
  heatCreated: boolean;
  oldest: number;
  newest: number;
}

interface EdgePass {
  drawnEdges: number;
  edgeCount: number;
  dimming: number;
}

export class GpuScene {
  readonly textures: NodeTextures;
  readonly labels: LabelSprites;
  private readonly gl: WebGL2RenderingContext;
  private readonly uniforms: UniformCache;
  private readonly nodeProgram: WebGLProgram;
  private readonly edgeProgram: WebGLProgram;
  private readonly quadBuffer: WebGLBuffer;
  private readonly edgeBuffer: WebGLBuffer;
  private readonly nodeVao: WebGLVertexArrayObject;
  private readonly edgeVao: WebGLVertexArrayObject;
  private ratio = 1;
  private halfWidth = 0.5;
  private halfHeight = 0.5;
  private deviceWidth = 1;
  private deviceHeight = 1;

  constructor(canvas: HTMLCanvasElement) {
    const gl = canvas.getContext('webgl2', {
      alpha: true,
      antialias: false,
      depth: false,
      powerPreference: 'default',
      preserveDrawingBuffer: false,
    });
    if (!gl) throw new Error(t('graph.errors.webgl2Unavailable'));
    this.gl = gl;
    this.uniforms = new UniformCache(gl);
    this.textures = new NodeTextures(gl);
    this.labels = new LabelSprites(gl);
    this.nodeProgram = createProgram(gl, NODE_VERTEX, NODE_FRAGMENT);
    this.edgeProgram = createProgram(gl, EDGE_VERTEX, EDGE_FRAGMENT);
    this.quadBuffer = createBuffer(gl);
    this.edgeBuffer = createBuffer(gl);
    this.nodeVao = createVao(gl);
    this.edgeVao = createVao(gl);
    gl.bindBuffer(gl.ARRAY_BUFFER, this.quadBuffer);
    gl.bufferData(gl.ARRAY_BUFFER, QUAD, gl.STATIC_DRAW);
    gl.disable(gl.DEPTH_TEST);
    gl.enable(gl.BLEND);
    gl.blendFuncSeparate(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA, gl.ONE, gl.ONE_MINUS_SRC_ALPHA);
    this.bindAttributes();
  }

  setViewport(width: number, height: number, ratio: number): void {
    this.ratio = ratio;
    this.deviceWidth = width;
    this.deviceHeight = height;
    this.halfWidth = width / 2;
    this.halfHeight = height / 2;
    this.gl.viewport(0, 0, width, height);
  }

  drawLabels(palette: Palette): void {
    this.labels.draw(this.deviceWidth, this.deviceHeight, this.ratio, palette);
  }

  uploadEdges(edges: Uint32Array): void {
    const gl = this.gl;
    gl.bindBuffer(gl.ARRAY_BUFFER, this.edgeBuffer);
    gl.bufferData(gl.ARRAY_BUFFER, edges, gl.STATIC_DRAW);
  }

  beginFrame(): void {
    const gl = this.gl;
    gl.clearColor(0, 0, 0, 0);
    gl.clear(gl.COLOR_BUFFER_BIT);
    this.textures.bindUnits();
  }

  drawEdges(view: CameraView, pass: EdgePass, palette: Palette): void {
    if (pass.drawnEdges <= 0) return;
    const gl = this.gl;
    const program = this.edgeProgram;
    gl.useProgram(program);
    this.setCamera(program, view);
    gl.uniform2f(
      this.location(program, 'uWidthLimits'),
      EDGE_MIN_HALF_PIXELS * this.ratio,
      EDGE_MAX_HALF_PIXELS * this.ratio,
    );
    gl.uniform1f(this.location(program, 'uDimming'), pass.dimming);
    gl.uniform4fv(
      this.location(program, 'uEdgeColor'),
      fadeEdges(palette.edge, pass.edgeCount),
    );
    gl.uniform4fv(this.location(program, 'uEdgeActive'), palette.edgeActive);
    gl.bindVertexArray(this.edgeVao);
    gl.drawArraysInstanced(gl.TRIANGLES, 0, 6, pass.drawnEdges);
    gl.bindVertexArray(null);
  }

  drawNodes(view: CameraView, pass: NodePass, palette: Palette): void {
    if (pass.nodeCount <= 0) return;
    const gl = this.gl;
    const program = this.nodeProgram;
    gl.useProgram(program);
    this.setCamera(program, view);
    gl.uniform1i(this.location(program, 'uFreshness'), FRESHNESS_UNIT);
    gl.uniform2f(
      this.location(program, 'uRadiusLimits'),
      SUB_PIXEL_RADIUS * this.ratio,
      MAX_NODE_PIXELS * this.ratio,
    );
    gl.uniform1f(this.location(program, 'uSizeScale'), pass.sizeScale);
    gl.uniform1f(this.location(program, 'uDimming'), pass.dimming);
    gl.uniform2f(this.location(program, 'uFreshnessRange'), pass.oldest, pass.newest);
    gl.uniform1f(this.location(program, 'uHeatmap'), pass.heatmap ? 1 : 0);
    gl.uniform1f(this.location(program, 'uHeatCreated'), pass.heatCreated ? 1 : 0);
    gl.uniform4fv(this.location(program, 'uFill'), palette.fill);
    gl.uniform4fv(this.location(program, 'uOutline'), palette.outline);
    gl.uniform4fv(this.location(program, 'uBackdrop'), palette.backdrop);
    gl.uniform4fv(this.location(program, 'uCold'), palette.cold);
    gl.uniform4fv(this.location(program, 'uHot'), palette.hot);
    gl.bindVertexArray(this.nodeVao);
    gl.drawArraysInstanced(gl.TRIANGLES, 0, 6, pass.nodeCount);
    gl.bindVertexArray(null);
  }

  dispose(): void {
    const gl = this.gl;
    gl.deleteBuffer(this.quadBuffer);
    gl.deleteBuffer(this.edgeBuffer);
    this.labels.dispose();
    this.textures.dispose();
    gl.deleteVertexArray(this.nodeVao);
    gl.deleteVertexArray(this.edgeVao);
    gl.deleteProgram(this.nodeProgram);
    gl.deleteProgram(this.edgeProgram);
    gl.getExtension('WEBGL_lose_context')?.loseContext();
  }

  private setCamera(program: WebGLProgram, view: CameraView): void {
    const gl = this.gl;
    gl.uniform2f(this.location(program, 'uCenter'), view.centerX, view.centerY);
    gl.uniform2f(this.location(program, 'uHalfViewport'), this.halfWidth, this.halfHeight);
    gl.uniform1f(this.location(program, 'uScale'), view.deviceScale);
    gl.uniform1f(this.location(program, 'uSpread'), view.spread);
    gl.uniform1i(this.location(program, 'uNodes'), NODE_UNIT);
    gl.uniform1i(this.location(program, 'uHighlight'), HIGHLIGHT_UNIT);
    gl.uniform1i(this.location(program, 'uReveal'), REVEAL_UNIT);
  }

  private location(program: WebGLProgram, name: string): WebGLUniformLocation | null {
    return this.uniforms.location(program, name);
  }

  private bindAttributes(): void {
    const gl = this.gl;
    gl.bindVertexArray(this.nodeVao);
    gl.bindBuffer(gl.ARRAY_BUFFER, this.quadBuffer);
    bindCorner(gl, this.nodeProgram);

    gl.bindVertexArray(this.edgeVao);
    gl.bindBuffer(gl.ARRAY_BUFFER, this.quadBuffer);
    bindCorner(gl, this.edgeProgram);
    gl.bindBuffer(gl.ARRAY_BUFFER, this.edgeBuffer);
    bindEdgeEndpoints(gl, this.edgeProgram);
    gl.bindVertexArray(null);
  }
}
