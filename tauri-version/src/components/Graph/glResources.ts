import { t } from '../../i18n';

export function createProgram(
  gl: WebGL2RenderingContext,
  vertex: string,
  fragment: string,
): WebGLProgram {
  const program = gl.createProgram();
  if (!program) throw new Error(t('graph.errors.programCreate'));
  attach(gl, program, gl.VERTEX_SHADER, vertex);
  attach(gl, program, gl.FRAGMENT_SHADER, fragment);
  gl.linkProgram(program);
  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
    const log = gl.getProgramInfoLog(program);
    gl.deleteProgram(program);
    throw new Error(t('graph.errors.programLink', { log: log ?? '' }));
  }
  return program;
}

function attach(
  gl: WebGL2RenderingContext,
  program: WebGLProgram,
  type: number,
  source: string,
): void {
  const shader = gl.createShader(type);
  if (!shader) throw new Error(t('graph.errors.shaderCreate'));
  gl.shaderSource(shader, source);
  gl.compileShader(shader);
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    const log = gl.getShaderInfoLog(shader);
    gl.deleteShader(shader);
    throw new Error(t('graph.errors.shaderCompile', { log: log ?? '' }));
  }
  gl.attachShader(program, shader);
  gl.deleteShader(shader);
}

export function createBuffer(gl: WebGL2RenderingContext): WebGLBuffer {
  const buffer = gl.createBuffer();
  if (!buffer) throw new Error(t('graph.errors.bufferCreate'));
  return buffer;
}

export function createVao(gl: WebGL2RenderingContext): WebGLVertexArrayObject {
  const vao = gl.createVertexArray();
  if (!vao) throw new Error(t('graph.errors.vaoCreate'));
  return vao;
}

export function createDataTexture(gl: WebGL2RenderingContext): WebGLTexture {
  const texture = gl.createTexture();
  if (!texture) throw new Error(t('graph.errors.textureCreate'));
  gl.bindTexture(gl.TEXTURE_2D, texture);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
  return texture;
}

export function bindCorner(gl: WebGL2RenderingContext, program: WebGLProgram): void {
  const location = gl.getAttribLocation(program, 'aCorner');
  if (location < 0) return;
  gl.enableVertexAttribArray(location);
  gl.vertexAttribPointer(location, 2, gl.FLOAT, false, 0, 0);
  gl.vertexAttribDivisor(location, 0);
}

export function bindEdgeEndpoints(gl: WebGL2RenderingContext, program: WebGLProgram): void {
  const location = gl.getAttribLocation(program, 'aEdge');
  if (location < 0) return;
  gl.enableVertexAttribArray(location);
  gl.vertexAttribIPointer(location, 2, gl.UNSIGNED_INT, 8, 0);
  gl.vertexAttribDivisor(location, 1);
}

export class UniformCache {
  private readonly cache = new Map<WebGLProgram, Map<string, WebGLUniformLocation | null>>();

  constructor(private readonly gl: WebGL2RenderingContext) {}

  location(program: WebGLProgram, name: string): WebGLUniformLocation | null {
    let names = this.cache.get(program);
    if (!names) {
      names = new Map();
      this.cache.set(program, names);
    }
    if (!names.has(name)) names.set(name, this.gl.getUniformLocation(program, name));
    return names.get(name) ?? null;
  }
}
