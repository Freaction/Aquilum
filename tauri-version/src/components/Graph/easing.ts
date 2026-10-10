export const LONGEST_FRAME_SECONDS = 0.1;
export const FADE_SECONDS = 0.07;
export const SETTLED_FADE = 0.02;

export function approachStep(seconds: number, timeConstant: number): number {
  return 1 - Math.exp(-Math.min(seconds, LONGEST_FRAME_SECONDS) / timeConstant);
}
