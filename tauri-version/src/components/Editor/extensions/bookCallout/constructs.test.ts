import { describe, expect, it } from 'vitest';
import { Text } from '@codemirror/state';
import { groupAdjacentBookCallouts } from './constructs';

const sizes = (source: string) => groupAdjacentBookCallouts(Text.of(source.split('\n')))
  .map((group) => group.items.length);

describe('groupAdjacentBookCallouts', () => {
  it('соединяет книги, идущие вплотную', () => {
    expect(sizes('> [!book] [[A]]\n> [!book] [[B]]')).toEqual([2]);
  });

  it('соединяет книги через пустую строку', () => {
    expect(sizes('> [!book] [[A]]\n\n> [!book] [[B]]')).toEqual([2]);
  });

  it('соединяет книги через пустую строку цитаты', () => {
    expect(sizes('> [!book] [[A]]\n>\n> [!book] [[B]]')).toEqual([2]);
  });

  it('соединяет книги с телом через пустую строку', () => {
    expect(sizes('> [!book] [[A]]\n> Автор: X\n\n> [!book] [[B]]\n> Автор: Y')).toEqual([2]);
  });

  it('соединяет многострочную книгу со следующей', () => {
    expect(sizes('> [!book] [[A]]\n> Автор: X\n> [!book] [[B]]')).toEqual([2]);
  });

  it('разделяет книги обычным текстом между ними', () => {
    expect(sizes('> [!book] [[A]]\n\nтекст\n\n> [!book] [[B]]')).toEqual([1, 1]);
  });
});
