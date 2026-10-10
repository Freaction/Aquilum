import { describe, expect, it } from 'vitest';
import { EditorState } from '@codemirror/state';
import type { ViewportCollector, ViewportScanContext } from './viewportScan';
import { hashtagCollector } from './hashtags';

function runLineCollector<T>(
  target: ViewportCollector<T>,
  doc: string,
  caret = 0,
): { from: number; to: number }[] {
  const state = EditorState.create({ doc, selection: { anchor: caret } });
  const context = { view: null, state, tree: null } as unknown as ViewportScanContext;
  const accumulator = target.begin(context);
  for (let number = 1; number <= state.doc.lines; number += 1) {
    target.enterLine?.(state.doc.line(number), accumulator, context);
  }
  const ranges: { from: number; to: number }[] = [];
  target.finish(accumulator, context).between(0, state.doc.length, (from, to) => {
    ranges.push({ from, to });
  });
  return ranges;
}

function tags(doc: string, caret = 0): string[] {
  return runLineCollector(hashtagCollector, doc, caret)
    .map(({ from, to }) => doc.slice(from, to));
}

describe('hashtags collector', () => {
  it('находит теги в начале строки, после пробела и в кириллице', () => {
    expect(tags('#один текст #два\n#три и #четыре', 6))
      .toEqual(['#один', '#два', '#три', '#четыре']);
  });

  it('считает курсор в нулевой позиции стоящим внутри тега с начала документа', () => {
    expect(tags('#тег текст')).toEqual([]);
  });

  it('не считает тегом решётку внутри слова', () => {
    expect(tags('foo#bar baz')).toEqual([]);
  });

  it('снимает подсветку с тега, внутри которого стоит курсор', () => {
    expect(tags('#один #два', 8)).toEqual(['#один']);
  });

  it('видит тег в начале второй строки, а не только через пробел', () => {
    expect(tags('текст\n#тег')).toEqual(['#тег']);
  });
});
