import { describe, expect, it } from 'vitest';
import { caretAtSameLine } from './caretAtSameLine';

describe('caretAtSameLine', () => {
  const previous = 'первая\nвторая\nтретья\n';

  it('keeps the caret on the same line and column', () => {
    const next = 'ПЕРВАЯ\nВТОРАЯ\nТРЕТЬЯ\n';
    const caret = previous.indexOf('вторая') + 3;
    expect(caretAtSameLine(previous, next, caret)).toBe(next.indexOf('ВТОРАЯ') + 3);
  });

  it('clamps to the end of a line that became shorter', () => {
    const next = 'первая\nвт\nтретья\n';
    const caret = previous.indexOf('вторая') + 6;
    expect(caretAtSameLine(previous, next, caret)).toBe(next.indexOf('вт') + 2);
  });

  it('falls back to the end of the document when the line disappeared', () => {
    const next = 'только одна строка';
    const caret = previous.indexOf('третья');
    expect(caretAtSameLine(previous, next, caret)).toBe(next.length);
  });

  it('handles the first line and out-of-range positions', () => {
    expect(caretAtSameLine(previous, 'другая\nвторая\n', 3)).toBe(3);
    expect(caretAtSameLine(previous, 'x', previous.length + 100)).toBe(1);
  });
});
