import { createBuffer, createProgram, createVao, UniformCache } from './glResources';
import { LabelAtlas, type AtlasSlot } from './labelAtlas';
import { ATLAS_UNIT } from './nodeTextures';
import type { Palette } from './palette';
import { LABEL_FRAGMENT, LABEL_VERTEX, QUAD } from './shaders';

const FLOATS_PER_LABEL = 10;
const HALO_CSS_PIXELS = 1.4;

export class LabelSprites {
  readonly atlas: LabelAtlas;
  private readonly program: WebGLProgram;
  private readonly uniforms: UniformCache;
  private readonly quadBuffer: WebGLBuffer;
  private readonly instanceBuffer: WebGLBuffer;
  private readonly vao: WebGLVertexArrayObject;
  private instances = new Float32Array(0);
  private count = 0;

  constructor(private readonly gl: WebGL2RenderingContext) {
    this.atlas = new LabelAtlas(gl);
    this.uniforms = new UniformCache(gl);
    this.program = createProgram(gl, LABEL_VERTEX, LABEL_FRAGMENT);
    this.quadBuffer = createBuffer(gl);
    this.instanceBuffer = createBuffer(gl);
    this.vao = createVao(gl);
    gl.bindVertexArray(this.vao);
    gl.bindBuffer(gl.ARRAY_BUFFER, this.quadBuffer);
    gl.bufferData(gl.ARRAY_BUFFER, QUAD, gl.STATIC_DRAW);
    this.bind('aCorner', 2, 0, 0, 0);
    gl.bindBuffer(gl.ARRAY_BUFFER, this.instanceBuffer);
    const stride = FLOATS_PER_LABEL * 4;
    this.bind('aRect', 4, stride, 0, 1);
    this.bind('aUv', 4, stride, 16, 1);
    this.bind('aStyle', 2, stride, 32, 1);
    gl.bindVertexArray(null);
  }

  reserve(capacity: number): void {
    const floats = capacity * FLOATS_PER_LABEL;
    if (this.instances.length >= floats) return;
    this.instances = new Float32Array(floats);
    const gl = this.gl;
    gl.bindBuffer(gl.ARRAY_BUFFER, this.instanceBuffer);
    gl.bufferData(gl.ARRAY_BUFFER, this.instances.byteLength, gl.DYNAMIC_DRAW);
  }

  begin(): void {
    this.count = 0;
    this.atlas.beginFrame();
  }

  place(
    slot: AtlasSlot,
    centerX: number,
    top: number,
    ratio: number,
    alpha: number,
    focused: boolean,
  ): void {
    const offset = this.count * FLOATS_PER_LABEL;
    if (offset + FLOATS_PER_LABEL > this.instances.length) return;
    const width = slot.cssWidth * ratio;
    const height = slot.cssHeight * ratio;
    const left = Math.round((centerX - slot.cssWidth / 2) * ratio);
    this.instances[offset] = left;
    this.instances[offset + 1] = Math.round(top * ratio);
    this.instances[offset + 2] = width;
    this.instances[offset + 3] = height;
    this.instances[offset + 4] = slot.u0;
    this.instances[offset + 5] = slot.v0;
    this.instances[offset + 6] = slot.u1;
    this.instances[offset + 7] = slot.v1;
    this.instances[offset + 8] = alpha;
    this.instances[offset + 9] = focused ? 1 : 0;
    this.count += 1;
  }

  draw(deviceWidth: number, deviceHeight: number, ratio: number, palette: Palette): void {
    if (this.count === 0) return;
    const gl = this.gl;
    gl.bindBuffer(gl.ARRAY_BUFFER, this.instanceBuffer);
    gl.bufferSubData(
      gl.ARRAY_BUFFER,
      0,
      this.instances.subarray(0, this.count * FLOATS_PER_LABEL),
    );
    gl.useProgram(this.program);
    this.atlas.bind(ATLAS_UNIT);
    gl.uniform1i(this.location('uAtlas'), ATLAS_UNIT);
    gl.uniform2f(this.location('uViewport'), deviceWidth, deviceHeight);
    const reach = (HALO_CSS_PIXELS * ratio) / this.atlas.size;
    gl.uniform2f(this.location('uHaloStep'), reach, reach);
    gl.uniform4fv(this.location('uFill'), palette.label);
    gl.uniform4fv(this.location('uFocus'), palette.labelFocus);
    gl.uniform4fv(this.location('uHalo'), palette.labelHalo);
    gl.bindVertexArray(this.vao);
    gl.drawArraysInstanced(gl.TRIANGLES, 0, 6, this.count);
    gl.bindVertexArray(null);
  }

  dispose(): void {
    const gl = this.gl;
    this.atlas.dispose();
    gl.deleteBuffer(this.quadBuffer);
    gl.deleteBuffer(this.instanceBuffer);
    gl.deleteVertexArray(this.vao);
    gl.deleteProgram(this.program);
  }

  private location(name: string): WebGLUniformLocation | null {
    return this.uniforms.location(this.program, name);
  }

  private bind(
    name: string,
    size: number,
    stride: number,
    offset: number,
    divisor: number,
  ): void {
    const gl = this.gl;
    const location = gl.getAttribLocation(this.program, name);
    if (location < 0) return;
    gl.enableVertexAttribArray(location);
    gl.vertexAttribPointer(location, size, gl.FLOAT, false, stride, offset);
    gl.vertexAttribDivisor(location, divisor);
  }
}
