import { Text } from '@codemirror/state';
import { describe, expect, it } from 'vitest';
import { findImageEmbeds } from './constructs';

function doc(...lines: string[]) {
  return Text.of(lines);
}

describe('findImageEmbeds', () => {
  it('finds a lone image line and spans exactly it', () => {
    const text = doc('текст', '![700](Files/a.jpg)', 'ещё');
    const spans = findImageEmbeds(text);

    expect(spans).toHaveLength(1);
    expect(spans[0]).toMatchObject({
      src: 'Files/a.jpg',
      width: 700,
      align: 'center',
      from: text.line(2).from,
      to: text.line(2).to,
    });
  });

  it('skips images that share a line with text', () => {
    expect(findImageEmbeds(doc('до ![700](a.jpg) после'))).toHaveLength(0);
  });

  it('keeps every image of a multi-image document', () => {
    const spans = findImageEmbeds(doc('![700](a.jpg)', '', '![300|left](b.png)'));

    expect(spans.map((span) => span.src)).toEqual(['a.jpg', 'b.png']);
    expect(spans[1]?.align).toBe('left');
  });

  it('ignores lines without an image link', () => {
    expect(findImageEmbeds(doc('# Заголовок', '- список', '[ссылка](a.jpg)'))).toHaveLength(0);
  });
});
