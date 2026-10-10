import { LONGEST_FRAME_SECONDS } from './easing';

export type FrameStep = (seconds: number, time: number) => boolean;

export class FrameClock {
  private handle = 0;
  private last = 0;

  get time(): number {
    return this.last;
  }

  request(step: FrameStep): void {
    if (this.handle) return;
    this.handle = requestAnimationFrame((time) => {
      this.handle = 0;
      const seconds = this.last
        ? Math.min((time - this.last) / 1000, LONGEST_FRAME_SECONDS)
        : 0;
      this.last = time;
      if (step(seconds, time)) this.request(step);
      if (!this.handle) this.last = 0;
    });
  }

  abandon(): void {
    this.handle = 0;
  }

  stop(): void {
    if (this.handle) cancelAnimationFrame(this.handle);
    this.handle = 0;
    this.last = 0;
  }
}
