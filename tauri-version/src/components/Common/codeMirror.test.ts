import { describe, expect, it } from 'vitest';
import { normalizeSingleLineText } from './codeMirror';

describe('normalizeSingleLineText', () => {
  it('flattens pasted line breaks and their indentation', () => {
    expect(normalizeSingleLineText('first\n    second\r\n\tthird'))
      .toBe('first second third');
  });

  it('preserves ordinary spaces in a single line', () => {
    expect(normalizeSingleLineText('first  second'))
      .toBe('first  second');
  });

  it('normalizes tabs without trimming surrounding text', () => {
    expect(normalizeSingleLineText(' before\t\tafter '))
      .toBe(' before after ');
  });
});
