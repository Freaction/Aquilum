import { createDataTexture } from './glResources';
import { NODE_TEXTURE_WIDTH } from './nodeMetrics';

export const NODE_UNIT = 0;
export const FRESHNESS_UNIT = 1;
export const HIGHLIGHT_UNIT = 2;
export const REVEAL_UNIT = 3;
export const ATLAS_UNIT = 4;

export class NodeTextures {
  private readonly nodes: WebGLTexture;
  private readonly freshness: WebGLTexture;
  private readonly highlight: WebGLTexture;
  private readonly reveal: WebGLTexture;

  constructor(private readonly gl: WebGL2RenderingContext) {
    this.nodes = createDataTexture(gl);
    this.freshness = createDataTexture(gl);
    this.highlight = createDataTexture(gl);
    this.reveal = createDataTexture(gl);
  }

  uploadNodes(texels: Float32Array, rows: number): void {
    const gl = this.gl;
    gl.bindTexture(gl.TEXTURE_2D, this.nodes);
    gl.texImage2D(
      gl.TEXTURE_2D,
      0,
      gl.RGBA32F,
      NODE_TEXTURE_WIDTH,
      rows,
      0,
      gl.RGBA,
      gl.FLOAT,
      texels,
    );
  }

  uploadFreshness(texels: Float32Array, rows: number): void {
    const gl = this.gl;
    gl.bindTexture(gl.TEXTURE_2D, this.freshness);
    gl.texImage2D(
      gl.TEXTURE_2D,
      0,
      gl.R32F,
      NODE_TEXTURE_WIDTH,
      rows,
      0,
      gl.RED,
      gl.FLOAT,
      texels,
    );
  }

  allocateHighlight(data: Uint8Array, rows: number): void {
    this.allocateBytes(this.highlight, data, rows);
  }

  uploadHighlightRows(data: Uint8Array, firstRow: number, rowCount: number): void {
    this.uploadRows(this.highlight, data, firstRow, rowCount);
  }

  allocateReveal(data: Uint8Array, rows: number): void {
    this.allocateBytes(this.reveal, data, rows);
  }

  uploadRevealRows(data: Uint8Array, firstRow: number, rowCount: number): void {
    this.uploadRows(this.reveal, data, firstRow, rowCount);
  }

  private allocateBytes(texture: WebGLTexture, data: Uint8Array, rows: number): void {
    const gl = this.gl;
    gl.bindTexture(gl.TEXTURE_2D, texture);
    gl.texImage2D(
      gl.TEXTURE_2D,
      0,
      gl.R8,
      NODE_TEXTURE_WIDTH,
      rows,
      0,
      gl.RED,
      gl.UNSIGNED_BYTE,
      data,
    );
  }

  private uploadRows(
    texture: WebGLTexture,
    data: Uint8Array,
    firstRow: number,
    rowCount: number,
  ): void {
    if (rowCount <= 0) return;
    const gl = this.gl;
    const offset = firstRow * NODE_TEXTURE_WIDTH;
    gl.bindTexture(gl.TEXTURE_2D, texture);
    gl.texSubImage2D(
      gl.TEXTURE_2D,
      0,
      0,
      firstRow,
      NODE_TEXTURE_WIDTH,
      rowCount,
      gl.RED,
      gl.UNSIGNED_BYTE,
      data.subarray(offset, offset + rowCount * NODE_TEXTURE_WIDTH),
    );
  }

  bindUnits(): void {
    const gl = this.gl;
    gl.activeTexture(gl.TEXTURE0 + NODE_UNIT);
    gl.bindTexture(gl.TEXTURE_2D, this.nodes);
    gl.activeTexture(gl.TEXTURE0 + FRESHNESS_UNIT);
    gl.bindTexture(gl.TEXTURE_2D, this.freshness);
    gl.activeTexture(gl.TEXTURE0 + HIGHLIGHT_UNIT);
    gl.bindTexture(gl.TEXTURE_2D, this.highlight);
    gl.activeTexture(gl.TEXTURE0 + REVEAL_UNIT);
    gl.bindTexture(gl.TEXTURE_2D, this.reveal);
  }

  dispose(): void {
    const gl = this.gl;
    gl.deleteTexture(this.nodes);
    gl.deleteTexture(this.freshness);
    gl.deleteTexture(this.highlight);
    gl.deleteTexture(this.reveal);
  }
}
