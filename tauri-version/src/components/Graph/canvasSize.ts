interface CanvasMetrics {
  cssWidth: number;
  cssHeight: number;
  deviceWidth: number;
  deviceHeight: number;
  ratio: number;
}

interface SizeReading {
  metrics: CanvasMetrics;
  resized: boolean;
}

export class CanvasSizer {
  private ratio = 1;
  private watching = false;
  private query: MediaQueryList | null = null;
  private readonly handleRatioChange = () => this.onRatioChange();

  constructor(
    private readonly canvas: HTMLCanvasElement,
    private readonly onRatioChange: () => void,
  ) {}

  read(): SizeReading {
    const ratio = window.devicePixelRatio || 1;
    const cssWidth = Math.max(1, this.canvas.clientWidth);
    const cssHeight = Math.max(1, this.canvas.clientHeight);
    const deviceWidth = Math.max(1, Math.round(this.canvas.clientWidth * ratio));
    const deviceHeight = Math.max(1, Math.round(this.canvas.clientHeight * ratio));
    const metrics = { cssWidth, cssHeight, deviceWidth, deviceHeight, ratio };
    if (
      this.canvas.width === deviceWidth
      && this.canvas.height === deviceHeight
      && ratio === this.ratio
    ) {
      return { metrics, resized: false };
    }
    if (ratio !== this.ratio || !this.watching) {
      this.ratio = ratio;
      this.watching = true;
      this.watchRatio();
    }
    this.canvas.width = deviceWidth;
    this.canvas.height = deviceHeight;
    return { metrics, resized: true };
  }

  dispose(): void {
    this.query?.removeEventListener('change', this.handleRatioChange);
    this.query = null;
  }

  private watchRatio(): void {
    this.query?.removeEventListener('change', this.handleRatioChange);
    this.query = window.matchMedia(`(resolution: ${this.ratio}dppx)`);
    this.query.addEventListener('change', this.handleRatioChange);
  }
}
