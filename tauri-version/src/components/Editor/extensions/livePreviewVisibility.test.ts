import { Text } from '@codemirror/state';
import { describe, expect, it } from 'vitest';
import { shouldRevealSyntax } from './livePreviewVisibility';

describe('shouldRevealSyntax', () => {
  const doc = Text.of(['[[link]]', '***']);

  it('covers both edges of a mark on the caret line', () => {
    const withTail = Text.of(['***x']);
    expect(shouldRevealSyntax(withTail, 0, 0, 3)).toBe(true);
    expect(shouldRevealSyntax(withTail, 3, 0, 3)).toBe(true);
    expect(shouldRevealSyntax(withTail, 4, 0, 3)).toBe(false);
  });

  it('never reveals marks on another line', () => {
    const wikiFrom = 0;
    const wikiTo = 8;
    const hrFrom = doc.line(2).from;
    expect(shouldRevealSyntax(doc, hrFrom, wikiFrom, wikiTo)).toBe(false);
    expect(shouldRevealSyntax(doc, 3, hrFrom, hrFrom + 3)).toBe(false);
  });

  it('keeps the owner zone open through the caret-after-last-char edge', () => {
    expect(shouldRevealSyntax(doc, 0, 0, 8)).toBe(true);
    expect(shouldRevealSyntax(doc, 3, 0, 8)).toBe(true);
    expect(shouldRevealSyntax(doc, 8, 0, 8)).toBe(true);
    expect(shouldRevealSyntax(doc, 9, 0, 8)).toBe(false);
  });

  it('caret on the next line reveals only that line owners', () => {
    expect(shouldRevealSyntax(doc, 9, 0, 8)).toBe(false);
    expect(shouldRevealSyntax(doc, 9, 9, 12)).toBe(true);
  });
});
