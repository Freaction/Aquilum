import { t } from '../../i18n';
import { AtlasCells } from './atlasCells';

const ATLAS_SIZE = 2048;
const PADDING_PIXELS = 4;
const MAX_TEXT_PIXELS = 120;

export interface AtlasSlot {
  u0: number;
  v0: number;
  u1: number;
  v1: number;
  cssWidth: number;
  cssHeight: number;
}

export class LabelAtlas {
  private readonly cells = new AtlasCells();
  private readonly slots: AtlasSlot[] = [];
  private readonly widths = new Map<string, number>();
  private readonly scratch: HTMLCanvasElement;
  private readonly context: CanvasRenderingContext2D;
  private readonly texture: WebGLTexture;
  private columns = 1;
  private cellWidth = 1;
  private cellHeight = 1;
  private font = '';
  private ratio = 0;
  private lineHeight = 0;
  private frame = 0;

  constructor(private readonly gl: WebGL2RenderingContext) {
    const scratch = document.createElement('canvas');
    const context = scratch.getContext('2d', { willReadFrequently: true });
    if (!context) throw new Error(t('graph.errors.labelCanvasUnavailable'));
    this.scratch = scratch;
    this.context = context;
    const texture = gl.createTexture();
    if (!texture) throw new Error(t('graph.errors.labelTextureCreate'));
    this.texture = texture;
    gl.bindTexture(gl.TEXTURE_2D, texture);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    gl.texStorage2D(gl.TEXTURE_2D, 1, gl.R8, ATLAS_SIZE, ATLAS_SIZE);
  }

  get capacity(): number {
    return this.cells.size;
  }

  get size(): number {
    return ATLAS_SIZE;
  }

  configure(font: string, ratio: number, lineHeight: number): void {
    if (font === this.font && ratio === this.ratio && lineHeight === this.lineHeight) return;
    this.font = font;
    this.ratio = ratio;
    this.lineHeight = lineHeight;
    this.widths.clear();
    const cssWidth = MAX_TEXT_PIXELS + PADDING_PIXELS * 2;
    const cssHeight = lineHeight + PADDING_PIXELS * 2;
    this.cellWidth = Math.max(1, Math.ceil(cssWidth * ratio));
    this.cellHeight = Math.max(1, Math.ceil(cssHeight * ratio));
    this.scratch.width = this.cellWidth;
    this.scratch.height = this.cellHeight;
    this.columns = Math.max(1, Math.floor(ATLAS_SIZE / this.cellWidth));
    const rows = Math.max(1, Math.floor(ATLAS_SIZE / this.cellHeight));
    this.slots.length = 0;
    this.cells.resize(this.columns * rows);
  }

  forget(): void {
    this.cells.forget();
    this.slots.length = 0;
  }

  beginFrame(): void {
    this.frame += 1;
  }

  measure(text: string): number {
    const known = this.widths.get(text);
    if (known !== undefined) return known;
    this.context.font = this.font;
    const width = this.context.measureText(text).width;
    this.widths.set(text, width);
    return width;
  }

  fit(title: string): string {
    const width = this.measure(title);
    if (width <= MAX_TEXT_PIXELS) return title;
    const keep = Math.max(1, Math.floor((title.length * MAX_TEXT_PIXELS) / width) - 1);
    return `${title.slice(0, keep).trimEnd()}…`;
  }

  slot(text: string): AtlasSlot | null {
    const claim = this.cells.claim(text, this.frame);
    if (!claim) return null;
    if (!claim.fresh) return this.slots[claim.cell];
    const slot = this.rasterize(text, claim.cell);
    this.slots[claim.cell] = slot;
    return slot;
  }

  bind(unit: number): void {
    const gl = this.gl;
    gl.activeTexture(gl.TEXTURE0 + unit);
    gl.bindTexture(gl.TEXTURE_2D, this.texture);
  }

  dispose(): void {
    this.gl.deleteTexture(this.texture);
  }

  private rasterize(text: string, cell: number): AtlasSlot {
    const gl = this.gl;
    const width = this.cellWidth;
    const height = this.cellHeight;
    const cellCssWidth = width / this.ratio;
    const context = this.context;
    context.setTransform(1, 0, 0, 1, 0, 0);
    context.clearRect(0, 0, width, height);
    context.setTransform(this.ratio, 0, 0, this.ratio, 0, 0);
    context.font = this.font;
    context.textAlign = 'center';
    context.textBaseline = 'top';
    context.fillStyle = '#ffffff';
    context.fillText(text, cellCssWidth / 2, PADDING_PIXELS);

    const pixels = context.getImageData(0, 0, width, height).data;
    const coverage = new Uint8Array(width * height);
    for (let index = 0; index < coverage.length; index += 1) {
      coverage[index] = pixels[index * 4 + 3];
    }
    const column = cell % this.columns;
    const row = Math.floor(cell / this.columns);
    gl.bindTexture(gl.TEXTURE_2D, this.texture);
    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
    gl.texSubImage2D(
      gl.TEXTURE_2D,
      0,
      column * width,
      row * height,
      width,
      height,
      gl.RED,
      gl.UNSIGNED_BYTE,
      coverage,
    );

    const spanCss = Math.min(this.measure(text) + PADDING_PIXELS * 2, cellCssWidth);
    const spanDevice = Math.ceil(spanCss * this.ratio);
    const left = column * width + Math.round((width - spanDevice) / 2);
    return {
      u0: left / ATLAS_SIZE,
      v0: (row * height) / ATLAS_SIZE,
      u1: (left + spanDevice) / ATLAS_SIZE,
      v1: (row * height + height) / ATLAS_SIZE,
      cssWidth: spanDevice / this.ratio,
      cssHeight: height / this.ratio,
    };
  }
}
