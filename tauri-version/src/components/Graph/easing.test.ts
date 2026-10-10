import { describe, expect, it } from 'vitest';
import { approachStep, LONGEST_FRAME_SECONDS } from './easing';

describe('approachStep', () => {
  it('lands at the same place whatever the frame rate is', () => {
    const target = 1;
    let smooth = 0;
    smooth += (target - smooth) * approachStep(0.008, 0.14);
    smooth += (target - smooth) * approachStep(0.008, 0.14);
    const stuttering = 0 + (target - 0) * approachStep(0.016, 0.14);

    expect(smooth).toBeCloseTo(stuttering, 12);
  });

  it('never lets one frame jump the whole way after a long stall', () => {
    expect(approachStep(10, 0.14)).toBe(approachStep(LONGEST_FRAME_SECONDS, 0.14));
    expect(approachStep(10, 0.14)).toBeLessThan(1);
  });
});
